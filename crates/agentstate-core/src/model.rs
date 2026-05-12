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

/// Records why a state change happened — who made it, what triggered it, and a human note.
/// All fields are optional so existing callers need no changes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Cause {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>, // agent ID making this change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<CommitId>, // commit hash that triggered this change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>, // human-readable reason
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_commit: Option<CommitId>, // previous commit hash for tamper-evident chain
    pub ts: DateTime<Utc>,
    pub commit_seq: u64, // monotonic per-namespace
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
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
    #[serde(default)]
    pub cause: Option<Cause>,
}

impl Object {
    pub fn new_with_seq(
        ns: Namespace,
        mut req: PutRequest,
        commit_seq: u64,
        prev_commit: Option<&CommitId>,
    ) -> Self {
        let id = req.id.take().unwrap_or_else(|| Ulid::new().to_string());
        let ts = Utc::now();
        // Include commit_seq and prev_commit in the hash seed so that any WAL
        // reordering or record replacement is cryptographically detectable.
        let mut seed = format!(
            "{}:{}:{}:{}:{}",
            &ns,
            &id,
            req.r#type,
            ts.to_rfc3339(),
            commit_seq
        );
        if let Some(prev) = prev_commit {
            seed.push(':');
            seed.push_str(prev);
        }
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
            prev_commit: prev_commit.cloned(),
            ts,
            commit_seq,
            cause: req.cause,
        }
    }
}
