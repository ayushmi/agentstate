use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Inbound challenge request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRequest {
    /// The specific inference step being challenged (None = challenge the whole claim).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenged_step: Option<usize>,
    /// Human-readable rationale for the challenge.
    pub reason: String,
    /// Source IDs or claim IDs that contradict the challenged step.
    #[serde(default)]
    pub counter_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeStatus {
    Open,
    Resolved,
    Rejected,
}

/// A stored challenge against a claim's proof — itself hash-chained into the WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub challenge_id: String,
    pub claim_id: String,
    pub proof_id: String,
    pub ns: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenged_step: Option<usize>,
    pub reason: String,
    pub counter_evidence: Vec<String>,
    pub status: ChallengeStatus,
    pub ts: DateTime<Utc>,
}

impl Challenge {
    pub fn new(
        claim_id: String,
        proof_id: String,
        ns: String,
        req: ChallengeRequest,
    ) -> Self {
        Self {
            challenge_id: Ulid::new().to_string(),
            claim_id,
            proof_id,
            ns,
            challenged_step: req.challenged_step,
            reason: req.reason,
            counter_evidence: req.counter_evidence,
            status: ChallengeStatus::Open,
            ts: Utc::now(),
        }
    }
}
