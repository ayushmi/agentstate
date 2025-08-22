use ciborium::ser;
use crc32c::crc32c;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prometheus::{Histogram, HistogramOpts, IntCounter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tokio::sync::{mpsc, oneshot};

const MAGIC: [u8; 4] = *b"ASTW";
const VER: u8 = 1;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecType {
    Put = 1,
    Delete = 2,
    LeaseAcquire = 3,
    LeaseRenew = 4,
    LeaseRelease = 5,
    Idempotency = 6,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RecBody {
    Put {
        ns: String,
        obj: serde_json::Value,
    },
    Delete {
        ns: String,
        id: String,
    },
    LeaseAcquire {
        ns: String,
        key: String,
        owner: String,
        token: u64,
        ttl: u64,
    },
    LeaseRenew {
        ns: String,
        key: String,
        owner: String,
        token: u64,
        ttl: u64,
    },
    LeaseRelease {
        ns: String,
        key: String,
        owner: String,
        token: u64,
    },
    Idempotency {
        ns: String,
        key: String,
        response: serde_json::Value,
        expires_ts: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalSegmentMeta {
    pub name: String,
    pub max_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub version: u32,
    pub current_snapshot: Option<String>,
    pub snapshot_bookmark: Option<u64>,
    pub last_seq: u64,
    pub current_segment: String,
    pub segments: Vec<WalSegmentMeta>,
}

pub struct WalSegment {
    pub path: PathBuf,
    file: File,
    pub bytes: u64,
}

pub struct WalWriter {
    dir: PathBuf,
    seg_size: u64,
    inner: Arc<RwLock<WalInner>>,
    tx: mpsc::Sender<Enq>,
}

#[derive(Clone)]
struct WalHandle {
    dir: PathBuf,
    seg_size: u64,
    inner: Arc<RwLock<WalInner>>,
}

struct WalInner {
    pub current_seq: u64,
    pub segment: WalSegment,
    pub manifest: Manifest,
}

struct Enq {
    rec: Vec<u8>,
    size: usize,
    seq: u64,
    ack: oneshot::Sender<()>,
}

static WAL_RECORDS_TOTAL: Lazy<IntCounter> =
    Lazy::new(|| IntCounter::new("wal_records_total", "wal records").unwrap());
static WAL_BYTES_TOTAL: Lazy<IntCounter> =
    Lazy::new(|| IntCounter::new("wal_bytes_total", "wal bytes").unwrap());
static WAL_FSYNC_TOTAL: Lazy<IntCounter> =
    Lazy::new(|| IntCounter::new("wal_fsync_total", "wal fsyncs").unwrap());
static WAL_BATCH_BYTES: Lazy<Histogram> = Lazy::new(|| {
    Histogram::with_opts(HistogramOpts::new("wal_batch_bytes", "wal batch sizes")).unwrap()
});
static WAL_FSYNC_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    Histogram::with_opts(HistogramOpts::new("wal_fsync_seconds", "wal fsync time")).unwrap()
});

impl WalWriter {
    pub fn open(dir: impl AsRef<Path>, seg_size: u64, start_seq: u64) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(dir.join("wal"))?;
        std::fs::create_dir_all(dir.join("snapshots"))?;
        let manifest_path = dir.join("manifest.json");
        let mut manifest: Manifest = if manifest_path.exists() {
            let s = std::fs::read_to_string(&manifest_path)?;
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            Manifest {
                version: 1,
                current_snapshot: None,
                snapshot_bookmark: None,
                last_seq: 0,
                current_segment: String::new(),
                segments: vec![],
            }
        };
        let seg_name = if manifest.current_segment.is_empty() {
            Self::new_segment_name(manifest.segments.last())
        } else {
            manifest.current_segment.clone()
        };
        let seg_path = dir.join("wal").join(&seg_name);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&seg_path)?;
        let bytes = file.metadata()?.len();
        let segment = WalSegment {
            path: seg_path,
            file,
            bytes,
        };
        if manifest.segments.is_empty() {
            manifest.segments.push(WalSegmentMeta {
                name: seg_name.clone(),
                max_seq: 0,
            });
        }
        manifest.current_segment = seg_name;
        let (tx, mut rx) = mpsc::channel::<Enq>(1024);
        // register metrics in default registry
        let reg = prometheus::default_registry();
        let _ = reg.register(Box::new(WAL_RECORDS_TOTAL.clone()));
        let _ = reg.register(Box::new(WAL_BYTES_TOTAL.clone()));
        let _ = reg.register(Box::new(WAL_FSYNC_TOTAL.clone()));
        let _ = reg.register(Box::new(WAL_BATCH_BYTES.clone()));
        let _ = reg.register(Box::new(WAL_FSYNC_SECONDS.clone()));

        let inner = Arc::new(RwLock::new(WalInner {
            current_seq: start_seq,
            segment,
            manifest,
        }));
        let me = Self {
            dir: dir.clone(),
            seg_size,
            inner: inner.clone(),
            tx,
        };
        let handle = WalHandle {
            dir: dir.clone(),
            seg_size,
            inner: inner.clone(),
        };
        tokio::spawn(async move {
            handle.fsync_worker(&mut rx).await;
        });
        Ok(me)
    }

    fn new_segment_name(prev: Option<&WalSegmentMeta>) -> String {
        if let Some(p) = prev {
            if let Ok(n) = p.name.trim_end_matches(".wal").parse::<u64>() {
                return format!("{:08}.wal", n + 1);
            }
        }
        "00000001.wal".to_string()
    }

    pub fn manifest(&self) -> Manifest {
        self.inner.read().manifest.clone()
    }

    pub async fn append(&self, seq: u64, ts: i64, body: &RecBody) -> std::io::Result<()> {
        let mut v = Vec::new();
        ser::into_writer(body, &mut v).unwrap();
        let len = v.len() as u32;
        let ns_hash = 0u64; // reserved
        let mut rec = Vec::with_capacity(4 + 1 + 1 + 8 + 8 + 8 + 4 + v.len() + 4);
        rec.extend_from_slice(&MAGIC);
        rec.push(VER);
        rec.push(Self::rectype(body) as u8);
        rec.extend_from_slice(&ns_hash.to_be_bytes());
        rec.extend_from_slice(&seq.to_be_bytes());
        rec.extend_from_slice(&(ts as u64).to_be_bytes());
        rec.extend_from_slice(&len.to_be_bytes());
        rec.extend_from_slice(&v);
        let crc = crc32c(&rec);
        rec.extend_from_slice(&(crc.to_be_bytes()));
        WAL_RECORDS_TOTAL.inc();
        WAL_BYTES_TOTAL.inc_by(rec.len() as u64);
        let (tx, rx) = oneshot::channel();
        let _ = self
            .tx
            .send(Enq {
                rec,
                size: len as usize,
                seq,
                ack: tx,
            })
            .await;
        let _ = rx.await; // wait fsync
        Ok(())
    }

    fn rectype(b: &RecBody) -> RecType {
        match b {
            RecBody::Put { .. } => RecType::Put,
            RecBody::Delete { .. } => RecType::Delete,
            RecBody::LeaseAcquire { .. } => RecType::LeaseAcquire,
            RecBody::LeaseRenew { .. } => RecType::LeaseRenew,
            RecBody::LeaseRelease { .. } => RecType::LeaseRelease,
            RecBody::Idempotency { .. } => RecType::Idempotency,
        }
    }
}

impl WalHandle {
    async fn fsync_worker(self, rx: &mut mpsc::Receiver<Enq>) {
        let batch_max = std::env::var("WAL_BATCH_MAX_BYTES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(256 * 1024);
        let batch_ms = std::env::var("WAL_BATCH_MAX_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3);
        while let Some(first) = rx.recv().await {
            let mut batch = vec![first];
            let mut bytes = batch[0].rec.len();
            let deadline = tokio::time::sleep(Duration::from_millis(batch_ms));
            tokio::pin!(deadline);
            loop {
                tokio::select! {
                    maybe = rx.recv() => { if let Some(enq) = maybe { bytes += enq.rec.len(); batch.push(enq); if bytes >= batch_max { break; } } else { break; } },
                    _ = &mut deadline => break,
                }
            }
            let t0 = std::time::Instant::now();
            {
                let mut inner = self.inner.write();
                for enq in &batch {
                    let _ = inner.segment.file.write_all(&enq.rec);
                }
                inner.segment.bytes += bytes as u64;
                let last_seq = batch
                    .last()
                    .map(|e| e.seq)
                    .unwrap_or(inner.manifest.last_seq);
                inner.manifest.last_seq = inner.manifest.last_seq.max(last_seq);
                if let Some(meta) = inner.manifest.segments.last_mut() {
                    meta.max_seq = meta.max_seq.max(last_seq);
                }
                let _ = inner.segment.file.flush();
                // Use sync_data instead of sync_all for better performance
                // sync_data only syncs file data, not metadata, which is faster
                let _ = inner.segment.file.sync_data();
                WAL_FSYNC_TOTAL.inc();
                WAL_FSYNC_SECONDS.observe(t0.elapsed().as_secs_f64());
                WAL_BATCH_BYTES.observe(bytes as f64);
                let _ = persist_manifest_at(&self.dir, &inner.manifest);
                // rotation
                let rotate_at = std::env::var("WAL_SEGMENT_BYTES")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(self.seg_size);
                if inner.segment.bytes >= rotate_at {
                    let _ = self.rotate_locked(&mut inner);
                }
                for enq in batch {
                    let _ = enq.ack.send(());
                }
            }
        }
    }

    fn rotate_locked(&self, inner: &mut WalInner) -> std::io::Result<()> {
        let name = WalWriter::new_segment_name(inner.manifest.segments.last());
        let seg_path = self.dir.join("wal").join(&name);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&seg_path)?;
        inner.segment = WalSegment {
            path: seg_path.clone(),
            file,
            bytes: 0,
        };
        inner.manifest.current_segment = name.clone();
        inner.manifest.segments.push(WalSegmentMeta {
            name,
            max_seq: inner.manifest.last_seq,
        });
        persist_manifest_at(&self.dir, &inner.manifest)
    }
}

fn persist_manifest_at(dir: &Path, m: &Manifest) -> std::io::Result<()> {
    let tmp = dir.join("manifest.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(m).unwrap())?;
    std::fs::rename(tmp, dir.join("manifest.json"))
}

pub fn replay(dir: impl AsRef<Path>) -> std::io::Result<Vec<RecBody>> {
    let dir = dir.as_ref().to_path_buf();
    let manifest_path = dir.join("manifest.json");
    let manifest: Manifest = if manifest_path.exists() {
        let s = std::fs::read_to_string(&manifest_path)?;
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        Manifest::default()
    };
    let mut out = Vec::new();
    for meta in manifest.segments.iter() {
        let name = &meta.name;
        let p = dir.join("wal").join(name);
        if let Ok(mut f) = File::open(&p) {
            loop {
                let mut hdr = [0u8; 4 + 1 + 1 + 8 + 8 + 8 + 4];
                if let Err(_) = f.read_exact(&mut hdr) {
                    break;
                }
                if &hdr[0..4] != MAGIC.as_ref() {
                    break;
                }
                let _ver = hdr[4];
                let _typ = hdr[5];
                let _ns_hash = u64::from_be_bytes(hdr[6..14].try_into().unwrap());
                let _seq = u64::from_be_bytes(hdr[14..22].try_into().unwrap());
                let _ts = u64::from_be_bytes(hdr[22..30].try_into().unwrap());
                let len = u32::from_be_bytes(hdr[30..34].try_into().unwrap()) as usize;
                let mut body = vec![0u8; len];
                if let Err(_) = f.read_exact(&mut body) {
                    break;
                }
                let mut crcbuf = [0u8; 4];
                if let Err(_) = f.read_exact(&mut crcbuf) {
                    break;
                }
                let mut rec = hdr.to_vec();
                rec.extend_from_slice(&body);
                let crc = crc32c(&rec);
                let got = u32::from_be_bytes(crcbuf);
                if crc != got {
                    break;
                }
                if let Ok(v) = ciborium::de::from_reader::<RecBody, _>(&body[..]) {
                    out.push(v);
                }
            }
        }
    }
    Ok(out)
}
