use crate::util::blake3_hex;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use ulid::Ulid;

pub type Namespace = String;
pub type ObjectId = String; // ULID string
pub type CommitId = String; // blake3 hex

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tags(pub BTreeMap<String, String>);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VecField {
    pub name: String,
    pub dims: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: ObjectId,
    pub ns: Namespace,
    pub r#type: String,
    pub body: JsonValue,
    #[serde(default)]
    pub tags: Tags,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
    #[serde(default)]
    pub parents: Vec<CommitId>,
    pub commit: CommitId,
    pub ts: DateTime<Utc>,
    pub commit_seq: u64, // monotonic per-namespace
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PutRequest {
    pub r#type: String,
    pub body: JsonValue,
    #[serde(default)]
    pub tags: Tags,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
    #[serde(default)]
    pub id: Option<ObjectId>,
    #[serde(default)]
    pub parents: Vec<CommitId>,
}

impl Object {
    pub fn new_with_seq(ns: Namespace, mut req: PutRequest, commit_seq: u64) -> Self {
        let id = req.id.take().unwrap_or_else(|| Ulid::new().to_string());
        let ts = Utc::now();
        let mut seed = format!("{}:{}:{}:{}", &ns, &id, req.r#type, ts.to_rfc3339());
        seed.push_str(&serde_json::to_string(&req.body).unwrap_or_default());
        let commit = blake3_hex(seed.as_bytes());
        Self {
            id,
            ns,
            r#type: req.r#type,
            body: req.body,
            tags: req.tags,
            ttl_seconds: req.ttl_seconds,
            parents: req.parents,
            commit,
            ts,
            commit_seq,
        }
    }
}
