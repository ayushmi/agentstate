use crate::walbin::{Manifest, RecBody, WalWriter};
use crate::{InMemoryStore, Storage};
use agentstate_core::{Object, PutRequest, QueryRequest, Result, StateError};
use chrono::Utc;
use std::{io::Write, path::PathBuf};
use ulid;
use zstd;

pub struct PersistentStore {
    mem: InMemoryStore,
    wal: parking_lot::Mutex<WalWriter>,
    manifest: parking_lot::RwLock<Manifest>,
    data_dir: PathBuf,
    idem: parking_lot::RwLock<
        std::collections::HashMap<(String, String), crate::traits::IdempotencyRecord>,
    >,
}

impl PersistentStore {
    pub fn open(data_dir: PathBuf) -> std::io::Result<Self> {
        let wal = WalWriter::open(&data_dir, 256 * 1024 * 1024, 0)?;
        let manifest = wal.manifest();
        // Replay existing WAL
        let recs = crate::walbin::replay(&data_dir).unwrap_or_default();
        let mem = InMemoryStore::new();
        let mut max_seq_per_ns: std::collections::HashMap<String, u64> = Default::default();
        for r in recs {
            match r {
                RecBody::Put { ns, obj } => {
                    if let Ok(mut o) = serde_json::from_value::<Object>(obj) {
                        // Track per-ns commit seq
                        max_seq_per_ns
                            .entry(o.ns.clone())
                            .and_modify(|m| *m = (*m).max(o.commit_seq))
                            .or_insert(o.commit_seq);
                        mem.replay_put(o);
                    }
                }
                RecBody::Delete { ns, id } => {
                    let seq = *max_seq_per_ns.get(&ns).unwrap_or(&0);
                    mem.replay_delete(&ns, &id, seq);
                }
                RecBody::LeaseAcquire { .. }
                | RecBody::LeaseRenew { .. }
                | RecBody::LeaseRelease { .. }
                | RecBody::Idempotency { .. } => {
                    // TODO: apply to lease and idempotency stores; left as an exercise for now
                }
            }
        }
        Ok(Self {
            mem,
            wal: parking_lot::Mutex::new(wal),
            manifest: parking_lot::RwLock::new(manifest),
            data_dir,
            idem: parking_lot::RwLock::new(std::collections::HashMap::new()),
        })
    }

    pub fn snapshot(&self) -> std::io::Result<String> {
        let ulid = ulid::Ulid::new().to_string();
        let path = self
            .data_dir
            .join("snapshots")
            .join(format!("snap-{}.zst", ulid));
        let file = std::fs::File::create(&path)?;
        let mut z = zstd::Encoder::new(file, 3)?;
        for o in self.mem.all_objects().into_iter() {
            let line = serde_json::to_string(&o).unwrap();
            z.write_all(line.as_bytes())?;
            z.write_all(b"\n")?;
        }
        z.finish()?;
        let mut m = self.manifest.write();
        m.current_snapshot = Some(path.file_name().unwrap().to_string_lossy().to_string());
        m.snapshot_bookmark = Some(m.last_seq);
        // persist manifest
        let manpath = self.data_dir.join("manifest.json");
        let tmp = self.data_dir.join("manifest.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&*m).unwrap())?;
        std::fs::rename(tmp, manpath)?;
        Ok(m.current_snapshot.clone().unwrap())
    }
}

#[async_trait::async_trait]
impl Storage for PersistentStore {
    async fn put(&self, ns: &str, req: PutRequest) -> Result<Object> {
        let o = self.mem.put(ns, req).await?;
        {
            let wal = self.wal.lock();
            let body = RecBody::Put {
                ns: o.ns.clone(),
                obj: serde_json::to_value(&o).unwrap(),
            };
            wal.append(o.commit_seq, Utc::now().timestamp(), &body)
                .await
                .map_err(|e| StateError::Internal(e.to_string()))?;
        }
        self.manifest.write().last_seq = self.manifest.read().last_seq.max(o.commit_seq);
        Ok(o)
    }

    async fn get(&self, ns: &str, id: &str, opts: crate::traits::GetOptions) -> Result<Object> {
        self.mem.get(ns, id, opts).await
    }
    async fn query(&self, ns: &str, req: QueryRequest) -> Result<Vec<Object>> {
        self.mem.query(ns, req).await
    }
    async fn delete(&self, ns: &str, id: &str) -> Result<()> {
        // Note: for durability correctness, capture commit seq before applying delete
        let now = Utc::now().timestamp();
        self.mem.delete(ns, id).await?;
        let body = RecBody::Delete {
            ns: ns.to_string(),
            id: id.to_string(),
        };
        let wal = self.wal.lock();
        wal.append(0, now, &body)
            .await
            .map_err(|e| StateError::Internal(e.to_string()))
    }

    fn subscribe(
        &self,
        filter: crate::traits::WatchFilter,
        from_commit: Option<u64>,
    ) -> Box<dyn crate::traits::WatchHandle> {
        self.mem.subscribe(filter, from_commit)
    }

    async fn sweep_expired(&self, retention_secs: u64) -> Result<u64> {
        self.mem.sweep_expired(retention_secs).await
    }

    async fn lease_acquire(
        &self,
        ns: &str,
        key: &str,
        owner: &str,
        ttl_secs: u64,
    ) -> Result<crate::traits::Lease> {
        let l = self.mem.lease_acquire(ns, key, owner, ttl_secs).await?;
        {
            let wal = self.wal.lock();
            wal.append(
                0,
                Utc::now().timestamp(),
                &RecBody::LeaseAcquire {
                    ns: ns.to_string(),
                    key: key.to_string(),
                    owner: owner.to_string(),
                    token: l.token,
                    ttl: ttl_secs,
                },
            )
            .await
            .map_err(|e| StateError::Internal(e.to_string()))?;
        }
        Ok(l)
    }
    async fn lease_renew(
        &self,
        ns: &str,
        key: &str,
        owner: &str,
        token: u64,
        ttl_secs: u64,
    ) -> Result<crate::traits::Lease> {
        let l = self
            .mem
            .lease_renew(ns, key, owner, token, ttl_secs)
            .await?;
        {
            let wal = self.wal.lock();
            wal.append(
                0,
                Utc::now().timestamp(),
                &RecBody::LeaseRenew {
                    ns: ns.to_string(),
                    key: key.to_string(),
                    owner: owner.to_string(),
                    token,
                    ttl: ttl_secs,
                },
            )
            .await
            .map_err(|e| StateError::Internal(e.to_string()))?;
        }
        Ok(l)
    }
    async fn lease_release(&self, ns: &str, key: &str, owner: &str, token: u64) -> Result<()> {
        self.mem.lease_release(ns, key, owner, token).await?;
        let wal = self.wal.lock();
        wal.append(
            0,
            Utc::now().timestamp(),
            &RecBody::LeaseRelease {
                ns: ns.to_string(),
                key: key.to_string(),
                owner: owner.to_string(),
                token,
            },
        )
        .await
        .map_err(|e| StateError::Internal(e.to_string()))
    }

    async fn idempotency_lookup(
        &self,
        ns: &str,
        key: &str,
        body_hash: &str,
    ) -> Result<Option<crate::traits::IdempotencyRecord>> {
        if let Some(r) = self.idem.read().get(&(ns.to_string(), key.to_string())) {
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
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<()> {
        let resp_hash =
            agentstate_core::util::blake3_hex(serde_json::to_string(&response).unwrap().as_bytes());
        let rec = crate::traits::IdempotencyRecord {
            ns: ns.to_string(),
            key: key.to_string(),
            body_hash: body_hash.to_string(),
            response_hash: resp_hash,
            commit_seq,
            expires_at,
            response: response.clone(),
        };
        self.idem
            .write()
            .insert((ns.to_string(), key.to_string()), rec.clone());
        let wal = self.wal.lock();
        wal.append(
            0,
            Utc::now().timestamp(),
            &RecBody::Idempotency {
                ns: ns.to_string(),
                key: key.to_string(),
                response,
                expires_ts: expires_at.timestamp(),
            },
        )
        .await
        .map_err(|e| StateError::Internal(e.to_string()))
    }
    async fn validate_fence(&self, ns: &str, resource: &str, fence: u64) -> Result<()> {
        self.mem.validate_fence(ns, resource, fence).await
    }

    async fn admin_snapshot(&self) -> Result<(String, u64)> {
        let id = self
            .snapshot()
            .map_err(|e| StateError::Internal(e.to_string()))?;
        let last = self.manifest.read().snapshot_bookmark.unwrap_or(0);
        Ok((id, last))
    }
    async fn admin_manifest(&self) -> Result<serde_json::Value> {
        let m = self.manifest.read().clone();
        Ok(serde_json::to_value(m).unwrap())
    }
    async fn admin_trim_wal(&self, snapshot_id: &str) -> Result<Vec<String>> {
        let mut m = self.manifest.write();
        if m.current_snapshot.as_deref() != Some(snapshot_id) {
            return Err(StateError::Invalid("snapshot id mismatch".into()));
        }
        let cutoff = m.snapshot_bookmark.unwrap_or(0);
        // retain segments: keep last one before cutoff for safety
        let mut deleted = Vec::new();
        let mut retain = Vec::new();
        let mut last_before_idx: Option<usize> = None;
        for (i, seg) in m.segments.iter().enumerate() {
            if seg.max_seq < cutoff {
                last_before_idx = Some(i);
            }
        }
        for (i, seg) in m.segments.iter().enumerate() {
            if seg.max_seq < cutoff {
                // delete unless it's the last_before_idx
                if Some(i) != last_before_idx {
                    let p = self.data_dir.join("wal").join(&seg.name);
                    let _ = std::fs::remove_file(&p);
                    deleted.push(seg.name.clone());
                } else {
                    retain.push(seg.clone());
                }
            } else {
                retain.push(seg.clone());
            }
        }
        m.segments = retain;
        // persist manifest
        let tmp = self.data_dir.join("manifest.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&*m).unwrap())
            .map_err(|e| StateError::Internal(e.to_string()))?;
        std::fs::rename(tmp, self.data_dir.join("manifest.json"))
            .map_err(|e| StateError::Internal(e.to_string()))?;
        Ok(deleted)
    }
}
