use crate::traits::{Storage, WatchEvent, WatchFilter, WatchHandle};
use agentstate_core::{Object, PutRequest, QueryRequest, Result, StateError};
use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prometheus::{register_histogram_vec, HistogramVec};
use std::collections::HashMap;
use std::sync::Arc;

static VECTOR_QUERY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!("vector_query_seconds", "ANN latency", &["field"]).unwrap()
});

#[derive(Clone)]
pub struct InMemoryStore {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Default)]
struct Inner {
    // key: (ns, id) -> versions (sorted oldest..newest)
    data: HashMap<(String, String), Vec<Object>>,
    // simple fanout list of per-ns buffers
    buffers: HashMap<String, Vec<WatchBuffer>>,
    // per-namespace commit counters and logs
    commit_seq: HashMap<String, u64>,
    commit_log: HashMap<String, Vec<WatchEvent>>, // events with commit_seq embedded
    // Secondary indexes on tags: (ns, tag_k, tag_v) -> set(ids as map for O(1))
    tag_index: HashMap<(String, String, String), HashMap<String, ()>>,
    // JSONPath (equality on materialized paths): (ns, path, value_json) -> ids
    json_index: HashMap<(String, String, String), HashMap<String, ()>>,
    // Registered json paths to index per ns
    json_index_paths: HashMap<String, Vec<String>>,
    // Leases: (ns,key) -> (owner, token, expires)
    leases: HashMap<(String, String), (String, u64, DateTime<Utc>)>,
    idem: HashMap<(String, String), super::traits::IdempotencyRecord>,
}

#[derive(Clone, Default)]
struct WatchBuffer {
    events: Arc<RwLock<Vec<WatchEvent>>>,
    cursor: Arc<RwLock<usize>>,  // consumer cursor
    bytes: Arc<RwLock<usize>>,   // approximate queued bytes
    overflow: Arc<RwLock<bool>>, // overflow flag
}

impl WatchBuffer {
    fn push(&self, ev: WatchEvent) {
        let max_events = std::env::var("WATCH_BUFFER_EVENTS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10_000);
        let max_bytes = std::env::var("WATCH_BUFFER_BYTES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(64 * 1024 * 1024);
        let approx = match &ev {
            WatchEvent::Put(o) => serde_json::to_vec(&o).map(|v| v.len()).unwrap_or(256),
            WatchEvent::Delete { .. } => 64,
        };
        let mut w = self.events.write();
        let mut b = self.bytes.write();
        if *b + approx > max_bytes || w.len() + 1 > max_events {
            *self.overflow.write() = true;
            return;
        }
        w.push(ev);
        *b += approx;
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::default())),
        }
    }

    fn is_expired(o: &Object, now: DateTime<Utc>) -> bool {
        if let Some(ttl) = o.ttl_seconds {
            o.ts + Duration::seconds(ttl as i64) < now
        } else {
            false
        }
    }

    pub fn replay_put(&self, obj: Object) {
        let mut inner = self.inner.write();
        let key = (obj.ns.clone(), obj.id.clone());
        inner.data.entry(key).or_default().push(obj.clone());
        for (k, v) in obj.tags.0.iter() {
            inner
                .tag_index
                .entry((obj.ns.clone(), k.clone(), v.clone()))
                .or_default()
                .insert(obj.id.clone(), ());
        }
        let paths = inner
            .json_index_paths
            .get(&obj.ns)
            .cloned()
            .unwrap_or_default();
        drop(inner);
        let mut inner = self.inner.write();
        for p in paths.iter() {
            if let Some(val) = obj.body.pointer(&json_pointer_from_path(p)) {
                let key = (obj.ns.clone(), p.clone(), val.to_string());
                inner
                    .json_index
                    .entry(key)
                    .or_default()
                    .insert(obj.id.clone(), ());
            }
        }
        inner
            .commit_log
            .entry(obj.ns.clone())
            .or_default()
            .push(WatchEvent::Put(obj));
    }

    pub fn replay_delete(&self, ns: &str, id: &str, commit_seq: u64) {
        let mut inner = self.inner.write();
        let key = (ns.to_string(), id.to_string());
        inner.data.remove(&key);
        inner
            .commit_log
            .entry(ns.to_string())
            .or_default()
            .push(WatchEvent::Delete {
                ns: ns.to_string(),
                id: id.to_string(),
                commit_seq,
            });
    }

    pub fn all_objects(&self) -> Vec<Object> {
        let inner = self.inner.read();
        let mut out = Vec::new();
        for ((_ns, _id), versions) in inner.data.iter() {
            if let Some(v) = versions.last() {
                out.push(v.clone());
            }
        }
        out
    }

    pub fn backlog_map(&self) -> std::collections::HashMap<String, usize> {
        let inner = self.inner.read();
        let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (ns, bufs) in inner.buffers.iter() {
            let mut maxv = 0usize;
            for b in bufs.iter() {
                let c = *b.cursor.read();
                let len = b.events.read().len();
                let backlog = len.saturating_sub(c);
                if backlog > maxv {
                    maxv = backlog;
                }
            }
            map.insert(ns.clone(), maxv);
        }
        map
    }
}

#[async_trait::async_trait(?Send)]
impl Storage for InMemoryStore {
    async fn put(&self, ns: &str, req: PutRequest) -> Result<Object> {
        let mut inner = self.inner.write();
        let next = inner
            .commit_seq
            .entry(ns.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        let commit_seq = *next;
        let obj = Object::new_with_seq(ns.to_string(), req, commit_seq);
        let key = (obj.ns.clone(), obj.id.clone());
        inner.data.entry(key.clone()).or_default().push(obj.clone());
        // maintain indexes
        for (k, v) in obj.tags.0.iter() {
            inner
                .tag_index
                .entry((obj.ns.clone(), k.clone(), v.clone()))
                .or_default()
                .insert(obj.id.clone(), ());
        }
        let paths_to_index = inner.json_index_paths.get(&obj.ns).cloned();
        if let Some(paths) = paths_to_index {
            for p in paths {
                if let Some(val) = obj.body.pointer(&json_pointer_from_path(&p)) {
                    let key = (obj.ns.clone(), p.clone(), val.to_string());
                    inner
                        .json_index
                        .entry(key)
                        .or_default()
                        .insert(obj.id.clone(), ());
                }
            }
        }
        // fanout/log
        let ev = WatchEvent::Put(obj.clone());
        inner
            .commit_log
            .entry(obj.ns.clone())
            .or_default()
            .push(ev.clone());
        // fanout to watchers for ns
        if let Some(bufs) = inner.buffers.get_mut(&obj.ns) {
            for b in bufs.iter() {
                b.push(WatchEvent::Put(obj.clone()));
            }
        }
        Ok(obj)
    }

    async fn get(&self, ns: &str, id: &str, opts: crate::traits::GetOptions) -> Result<Object> {
        let now = Utc::now();
        let inner = self.inner.read();
        let key = (ns.to_string(), id.to_string());
        let versions = inner.data.get(&key).ok_or(StateError::NotFound)?;
        let mut cand = None;
        for v in versions.iter().rev() {
            if let Some(at) = opts.at_ts {
                if v.ts > at {
                    continue;
                }
            }
            if !Self::is_expired(v, now) {
                cand = Some(v.clone());
                break;
            }
        }
        cand.ok_or(StateError::NotFound)
    }

    async fn query(&self, ns: &str, req: QueryRequest) -> Result<Vec<Object>> {
        let now = Utc::now();
        let inner = self.inner.read();
        let mut candidate_ids: Option<HashMap<String, ()>> = None;
        // tag index intersect
        if let Some(tf) = &req.tag_filter {
            for (k, v) in tf.0.iter() {
                let key = (ns.to_string(), k.clone(), v.clone());
                if let Some(ids) = inner.tag_index.get(&key) {
                    candidate_ids = Some(match candidate_ids.take() {
                        None => ids.clone(),
                        Some(prev) => prev
                            .into_iter()
                            .filter(|(id, _)| ids.contains_key(id))
                            .collect(),
                    });
                } else {
                    return Ok(vec![]);
                }
            }
        }
        // json index intersect
        if let Some(jf) = &req.jsonpath {
            for (p, val) in jf.equals.iter() {
                let key = (ns.to_string(), p.clone(), val.to_string());
                if let Some(ids) = inner.json_index.get(&key) {
                    candidate_ids = Some(match candidate_ids.take() {
                        None => ids.clone(),
                        Some(prev) => prev
                            .into_iter()
                            .filter(|(id, _)| ids.contains_key(id))
                            .collect(),
                    });
                } else {
                    return Ok(vec![]);
                }
            }
        }
        // Scan candidates or full ns
        let mut out = Vec::new();
        match candidate_ids {
            Some(ids) => {
                for (id, _) in ids.into_iter() {
                    if let Some(versions) = inner.data.get(&(ns.to_string(), id.clone())) {
                        if let Some(v) = versions.last() {
                            if !Self::is_expired(v, now) {
                                out.push(v.clone());
                            }
                        }
                    }
                }
            }
            None => {
                for ((n, _id), versions) in inner.data.iter() {
                    if n != ns {
                        continue;
                    }
                    if let Some(v) = versions.last() {
                        if !Self::is_expired(v, now) {
                            out.push(v.clone());
                        }
                    }
                }
            }
        }
        // Vector ANN naive filter over out
        if let Some(vq) = &req.vector {
            let _timer = VECTOR_QUERY_SECONDS
                .with_label_values(&[&vq.field])
                .start_timer();
            let mut scored: Vec<(f32, Object)> = Vec::new();
            for o in out.into_iter() {
                if let Some(vec_val) = o.body.get(&vq.field).and_then(|v| v.as_array()) {
                    let v: Vec<f32> = vec_val
                        .iter()
                        .filter_map(|x| x.as_f64().map(|f| f as f32))
                        .collect();
                    if v.len() == vq.embedding.len() {
                        let s = cosine_sim(&v, &vq.embedding);
                        scored.push((s, o));
                    }
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let top_k = vq.top_k.min(scored.len());
            let mut res = Vec::with_capacity(top_k);
            for i in 0..top_k {
                res.push(scored[i].1.clone());
            }
            return Ok(res);
        }
        if let Some(l) = req.limit {
            out.truncate(l);
        }
        Ok(out)
    }

    async fn delete(&self, ns: &str, id: &str) -> Result<()> {
        let mut inner = self.inner.write();
        let key = (ns.to_string(), id.to_string());
        let existed = inner.data.remove(&key).is_some();
        if existed {
            let seq = inner
                .commit_seq
                .entry(ns.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            let commit_seq = *seq;
            let ev = WatchEvent::Delete {
                ns: ns.to_string(),
                id: id.to_string(),
                commit_seq,
            };
            inner
                .commit_log
                .entry(ns.to_string())
                .or_default()
                .push(ev.clone());
            if let Some(bufs) = inner.buffers.get_mut(ns) {
                for b in bufs.iter() {
                    b.push(ev.clone());
                }
            }
            Ok(())
        } else {
            Err(StateError::NotFound)
        }
    }

    fn subscribe(
        &self,
        filter: WatchFilter,
        from_commit: Option<u64>,
    ) -> Box<dyn crate::traits::WatchHandle> {
        let mut inner = self.inner.write();
        let buf = WatchBuffer::default();
        // Prime the buffer with backlog since from_commit
        if let Some(from) = from_commit {
            if let Some(log) = inner.commit_log.get(&filter.ns) {
                for ev in log.iter() {
                    let c = match ev {
                        WatchEvent::Put(o) => o.commit_seq,
                        WatchEvent::Delete { commit_seq, .. } => *commit_seq,
                    };
                    if c > from {
                        buf.push(ev.clone());
                    }
                }
            }
        }
        inner
            .buffers
            .entry(filter.ns.clone())
            .or_default()
            .push(buf.clone());
        Box::new(MemWatch {
            buf,
            ns: filter.ns,
            last_commit: from_commit.unwrap_or(0),
        })
    }

    async fn sweep_expired(&self, _retention_secs: u64) -> Result<u64> {
        let now = Utc::now();
        let mut inner = self.inner.write();
        let mut removed = 0u64;
        let keys: Vec<(String, String)> = inner.data.keys().cloned().collect();
        for k in keys {
            if let Some(vec) = inner.data.get_mut(&k) {
                if let Some(last) = vec.last() {
                    if last
                        .ttl_seconds
                        .map(|t| last.ts + Duration::seconds(t as i64) < now)
                        .unwrap_or(false)
                    {
                        let dead = vec.last().cloned();
                        inner.data.remove(&k);
                        if let Some(o) = dead {
                            removed += 1;
                            drop(inner);
                            self.cleanup_indexes_for(&o).await;
                            inner = self.inner.write();
                        }
                    }
                }
            }
        }
        Ok(removed)
    }

    async fn lease_acquire(
        &self,
        ns: &str,
        key: &str,
        owner: &str,
        ttl_secs: u64,
    ) -> Result<crate::traits::Lease> {
        let mut inner = self.inner.write();
        let now = Utc::now();
        let expires = now + Duration::seconds(ttl_secs as i64);
        let entry = inner
            .leases
            .get(&(ns.to_string(), key.to_string()))
            .cloned();
        let token = {
            let token_ref = inner
                .commit_seq
                .entry(ns.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
            *token_ref
        };
        if let Some((cur_owner, _tok, cur_exp)) = entry {
            if cur_exp > now && cur_owner != owner {
                return Err(StateError::Conflict("lease held".into()));
            }
        }
        inner.leases.insert(
            (ns.to_string(), key.to_string()),
            (owner.to_string(), token, expires),
        );
        Ok(crate::traits::Lease {
            ns: ns.to_string(),
            key: key.to_string(),
            owner: owner.to_string(),
            token: token,
            expires_at: expires,
        })
    }

    async fn lease_renew(
        &self,
        ns: &str,
        key: &str,
        owner: &str,
        token: u64,
        ttl_secs: u64,
    ) -> Result<crate::traits::Lease> {
        let mut inner = self.inner.write();
        let now = Utc::now();
        let expires = now + Duration::seconds(ttl_secs as i64);
        match inner.leases.get_mut(&(ns.to_string(), key.to_string())) {
            Some((cur_owner, cur_tok, cur_exp)) => {
                if *cur_owner != owner || *cur_tok != token {
                    return Err(StateError::Conflict("fencing".into()));
                }
                *cur_exp = expires;
                Ok(crate::traits::Lease {
                    ns: ns.to_string(),
                    key: key.to_string(),
                    owner: owner.to_string(),
                    token,
                    expires_at: expires,
                })
            }
            None => Err(StateError::NotFound),
        }
    }

    async fn lease_release(&self, ns: &str, key: &str, owner: &str, token: u64) -> Result<()> {
        let mut inner = self.inner.write();
        match inner.leases.remove(&(ns.to_string(), key.to_string())) {
            Some((cur_owner, cur_tok, _)) if cur_owner == owner && cur_tok == token => Ok(()),
            Some((cur_owner, cur_tok, exp)) => {
                inner
                    .leases
                    .insert((ns.to_string(), key.to_string()), (cur_owner, cur_tok, exp));
                Err(StateError::Conflict("fencing".into()))
            }
            None => Err(StateError::NotFound),
        }
    }

    async fn idempotency_lookup(
        &self,
        ns: &str,
        key: &str,
        body_hash: &str,
    ) -> Result<Option<super::traits::IdempotencyRecord>> {
        let inner = self.inner.read();
        if let Some(r) = inner.idem.get(&(ns.to_string(), key.to_string())) {
            if r.body_hash == body_hash {
                return Ok(Some(r.clone()));
            } else {
                return Err(StateError::Conflict("idempotency body mismatch".into()));
            }
        }
        Ok(None)
    }
    async fn idempotency_commit(
        &self,
        ns: &str,
        key: &str,
        body_hash: &str,
        response: serde_json::Value,
        commit_seq: u64,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        let resp_hash =
            agentstate_core::util::blake3_hex(serde_json::to_string(&response).unwrap().as_bytes());
        let rec = super::traits::IdempotencyRecord {
            ns: ns.to_string(),
            key: key.to_string(),
            body_hash: body_hash.to_string(),
            response_hash: resp_hash,
            commit_seq,
            expires_at,
            response,
        };
        self.inner
            .write()
            .idem
            .insert((ns.to_string(), key.to_string()), rec);
        Ok(())
    }
    async fn validate_fence(&self, ns: &str, resource: &str, fence: u64) -> Result<()> {
        let inner = self.inner.read();
        if let Some((_, tok, exp)) = inner.leases.get(&(ns.to_string(), resource.to_string())) {
            if *tok == fence && *exp > Utc::now() {
                return Ok(());
            }
        }
        Err(StateError::Conflict("fence mismatch".into()))
    }
    async fn admin_snapshot(&self) -> Result<(String, u64)> {
        Err(StateError::Invalid("not persistent".into()))
    }
    async fn admin_manifest(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"mode":"memory"}))
    }
    async fn admin_trim_wal(&self, _snapshot_id: &str) -> Result<Vec<String>> {
        Err(StateError::Invalid("not persistent".into()))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct MemWatch {
    buf: WatchBuffer,
    ns: String,
    last_commit: u64,
}

impl WatchHandle for MemWatch {
    fn try_next(&mut self) -> Option<WatchEvent> {
        let mut c = self.buf.cursor.write();
        let events = self.buf.events.read();
        if *c < events.len() {
            let ev = events[*c].clone();
            *c += 1;
            // reduce bytes count approximately
            {
                let mut b = self.buf.bytes.write();
                *b = b.saturating_sub(match &ev {
                    WatchEvent::Put(o) => serde_json::to_vec(o).map(|v| v.len()).unwrap_or(256),
                    _ => 64,
                });
            }
            self.last_commit = match &ev {
                WatchEvent::Put(o) => o.commit_seq,
                WatchEvent::Delete { commit_seq, .. } => *commit_seq,
            };
            Some(ev)
        } else {
            None
        }
    }

    fn last_commit(&self) -> u64 {
        self.last_commit
    }

    fn overflow_meta(&self) -> Option<(u64, u32)> {
        if *self.buf.overflow.read() {
            let min = std::env::var("WATCH_RETRY_MIN_MS")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(250);
            let max = std::env::var("WATCH_RETRY_MAX_MS")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(4000);
            let retry = ((min as u64 + max as u64) / 2) as u32;
            Some((self.last_commit, retry))
        } else {
            None
        }
    }
}

fn json_pointer_from_path(path: &str) -> String {
    // Very naive: convert $.a.b -> /a/b ; $.items[0].id not supported yet
    let p = path.trim();
    let p = p.trim_start_matches('$').trim_start_matches('.');
    let parts: Vec<&str> = p.split('.').collect();
    let mut out = String::new();
    for part in parts {
        if !part.is_empty() {
            out.push('/');
            out.push_str(part);
        }
    }
    out
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

impl InMemoryStore {
    async fn cleanup_indexes_for(&self, obj: &Object) {
        let mut inner = self.inner.write();
        for (k, v) in obj.tags.0.iter() {
            if let Some(set) = inner
                .tag_index
                .get_mut(&(obj.ns.clone(), k.clone(), v.clone()))
            {
                set.remove(&obj.id);
            }
        }
        let paths_to_cleanup = inner.json_index_paths.get(&obj.ns).cloned();
        if let Some(paths) = paths_to_cleanup {
            for p in paths {
                if let Some(val) = obj.body.pointer(&json_pointer_from_path(&p)) {
                    if let Some(set) =
                        inner
                            .json_index
                            .get_mut(&(obj.ns.clone(), p.clone(), val.to_string()))
                    {
                        set.remove(&obj.id);
                    }
                }
            }
        }
    }
}
