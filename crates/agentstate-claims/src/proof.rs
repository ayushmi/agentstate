use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    Proving,
    Proved,
    Refuted,
    Inconclusive,
    Challenged,
}

/// The six formal properties every proof must satisfy, plus Lean machine verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofProperties {
    /// No claim C and ¬C are both derivable from this claim set.
    pub self_consistent: bool,
    /// Every declared premise is load-bearing — removing it breaks the proof.
    pub minimal: bool,
    /// At least one testable consequence is declared.
    pub has_predictive_constraint: bool,
    /// The proof DAG is complete and replayable.
    pub verifiable: bool,
    /// Every inference step is licensed by a declared domain rule.
    pub sound: bool,
    /// All WAL state premises are pinned to a specific commit (not floating).
    pub monotonic: bool,
    /// The proof certificate was accepted by the Lean 4 kernel (Tier 2+).
    /// False when Lean is unavailable or the domain has no formal.lean companion.
    pub machine_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// Leaf node: grounded in an external source or WAL state.
    Ground,
    /// Internal node: derived from prior steps by a domain rule.
    Inference,
    /// Root node: the final proved conclusion.
    Conclusion,
}

/// One node in the proof DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceStep {
    pub step_id: usize,
    pub kind: StepKind,
    /// The rule applied, e.g. "healthcare/v1:allergy_clearance"
    pub rule: String,
    /// IDs of prior steps this step depends on.
    pub premises_used: Vec<usize>,
    pub conclusion_predicate: String,
    pub conclusion_desc: String,
    /// For ground steps: the assumption ID that justifies this step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justified_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsequenceResult {
    pub predicate_desc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_after_hours: Option<u64>,
    /// "pending" | "holds" | "violated"
    pub status: String,
}

/// A first-class proof artifact — hash-chained, inspectable, challengeable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub proof_id: String,
    pub claim_id: String,
    pub ns: String,
    pub domain: String,
    pub status: ProofStatus,
    /// Formal property checks — all six must be true for the proof to be fully valid.
    pub properties: ProofProperties,
    /// The proved assertion (serialized ClaimAssertion).
    pub conclusion: serde_json::Value,
    /// "certain" | "probable" | "none"
    pub confidence: String,
    /// The proof DAG, in topological order (leaves first, root last).
    pub steps: Vec<InferenceStep>,
    /// Every assumption the proof depends on.
    pub assumptions: Vec<String>,
    pub consequences_checked: Vec<ConsequenceResult>,
    /// IDs of challenges submitted against this proof.
    pub challenges: Vec<String>,
    /// Populated only when status == Refuted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refutation_reason: Option<String>,
    pub ts: DateTime<Utc>,
    pub commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_commit: Option<String>,
    pub commit_seq: u64,
    /// Optional expiry time — status becomes "expired" after this timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
    /// Signers required before status advances from Proving to Proved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_signers: Vec<String>,
    /// Lean 4 proof certificate — a `.lean` file that the Lean kernel can verify.
    /// Present when the domain has a formal.lean companion (Tier 2+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lean_certificate: Option<String>,
}
