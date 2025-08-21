use agentstate_core::{Object, PutRequest, QueryRequest, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct GetOptions {
    pub at_ts: Option<DateTime<Utc>>, // time-travel
}

#[derive(Debug, Clone)]
pub struct WatchFilter {
    pub ns: String,
}

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Put(Object),
    Delete {
        ns: String,
        id: String,
        commit_seq: u64,
    },
}

#[async_trait::async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn put(&self, ns: &str, req: PutRequest) -> Result<Object>;
    async fn get(&self, ns: &str, id: &str, opts: GetOptions) -> Result<Object>;
    async fn query(&self, ns: &str, req: QueryRequest) -> Result<Vec<Object>>;
    async fn delete(&self, ns: &str, id: &str) -> Result<()>;
    async fn sweep_expired(&self, retention_secs: u64) -> Result<u64>; // returns removed count

    // Subscribe from an optional resume token (commit_seq)
    fn subscribe(&self, filter: WatchFilter, from_commit: Option<u64>) -> Box<dyn WatchHandle>;

    // Leases
    async fn lease_acquire(&self, ns: &str, key: &str, owner: &str, ttl_secs: u64)
        -> Result<Lease>;
    async fn lease_renew(
        &self,
        ns: &str,
        key: &str,
        owner: &str,
        token: u64,
        ttl_secs: u64,
    ) -> Result<Lease>;
    async fn lease_release(&self, ns: &str, key: &str, owner: &str, token: u64) -> Result<()>;

    // Idempotency
    async fn idempotency_lookup(
        &self,
        ns: &str,
        key: &str,
        body_hash: &str,
    ) -> Result<Option<IdempotencyRecord>>;
    async fn idempotency_commit(
        &self,
        ns: &str,
        key: &str,
        body_hash: &str,
        response: serde_json::Value,
        commit_seq: u64,
        expires_at: DateTime<Utc>,
    ) -> Result<()>;

    // Fence validation for writes
    async fn validate_fence(&self, ns: &str, resource: &str, fence: u64) -> Result<()>;

    // Admin
    async fn admin_snapshot(&self) -> Result<(String, u64)>;
    async fn admin_manifest(&self) -> Result<serde_json::Value>;
    async fn admin_trim_wal(&self, snapshot_id: &str) -> Result<Vec<String>>;
    
    // Backlog monitoring
    fn backlog_map(&self) -> std::collections::HashMap<String, u64> {
        Default::default()
    }
    
    // Export all objects (for admin dump)
    fn all_objects(&self) -> Vec<Object> {
        Vec::new()
    }

}

pub trait WatchHandle: Send {
    fn try_next(&mut self) -> Option<WatchEvent>;
    fn last_commit(&self) -> u64;
    fn overflow_meta(&self) -> Option<(u64, u32)>; // (last_commit, retry_after_ms)
}

#[derive(Debug, Clone)]
pub struct Lease {
    pub ns: String,
    pub key: String,
    pub owner: String,
    pub token: u64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    pub ns: String,
    pub key: String,
    pub body_hash: String,
    pub response_hash: String,
    pub commit_seq: u64,
    pub expires_at: DateTime<Utc>,
    pub response: serde_json::Value,
}
