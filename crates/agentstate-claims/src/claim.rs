use agentstate_core::{util::blake3_hex, Cause};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// The core assertion being made — predicate + subject + object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAssertion {
    /// e.g. "safe_to_prescribe", "entity_solvent", "contract_valid"
    pub predicate: String,
    /// What the claim is about (patient, entity, contract id…)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<serde_json::Value>,
    /// What is being predicated about the subject (drug, counterparty…)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<serde_json::Value>,
    /// Additional domain-specific parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A reference to a piece of evidence that grounds the claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PremiseRef {
    /// An external source (publication, database, document).
    Source { id: String, role: String },
    /// A specific version of a WAL object (pinned at a commit for monotonicity).
    WalState {
        ns: String,
        object_id: String,
        field: String,
        role: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        at_commit: Option<String>,
    },
    /// An axiom declared in the domain pack.
    DomainAxiom { axiom_id: String, role: String },
    /// A previously proved claim whose conclusion supports this one.
    PriorClaim { claim_id: String, role: String },
}

impl PremiseRef {
    /// The semantic role this premise plays in the proof.
    pub fn role(&self) -> &str {
        match self {
            PremiseRef::Source { role, .. } => role,
            PremiseRef::WalState { role, .. } => role,
            PremiseRef::DomainAxiom { role, .. } => role,
            PremiseRef::PriorClaim { role, .. } => role,
        }
    }

    /// A stable identifier for this assumption in the proof DAG.
    pub fn assumption_id(&self) -> String {
        match self {
            PremiseRef::Source { id, .. } => format!("source:{}", id),
            PremiseRef::WalState {
                ns,
                object_id,
                at_commit,
                ..
            } => {
                if let Some(c) = at_commit {
                    format!("wal:{}:{}@{}", ns, object_id, &c[..c.len().min(8)])
                } else {
                    format!("wal:{}:{}", ns, object_id)
                }
            }
            PremiseRef::DomainAxiom { axiom_id, .. } => format!("axiom:{}", axiom_id),
            PremiseRef::PriorClaim { claim_id, .. } => format!("claim:{}", claim_id),
        }
    }
}

/// A testable consequence: something that must hold in WAL state as a result of this claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consequence {
    /// Predicate in the invariant DSL format (same as Phase 2 invariants).
    pub predicate: serde_json::Value,
    /// Defer checking until N hours after the claim is made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_after_hours: Option<u64>,
    #[serde(default)]
    pub status: ConsequenceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConsequenceStatus {
    #[default]
    Pending,
    Holds,
    Violated,
}

/// Bounds a claim in space (namespace) and time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimScope {
    pub namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
}

/// Inbound request to submit a new claim for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRequest {
    /// Domain pack identifier, e.g. "healthcare/v1"
    pub domain: String,
    /// Template within the domain pack, e.g. "drug_safety"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// The assertion being made.
    pub assertion: ClaimAssertion,
    /// Evidence grounding this claim.
    #[serde(default)]
    pub premises: Vec<PremiseRef>,
    /// Testable consequences that must hold after this claim is accepted.
    #[serde(default)]
    pub consequences: Vec<Consequence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ClaimScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
}

impl ClaimRequest {
    /// Convert an inbound request into a stored, hash-chained claim.
    pub fn into_claim(self, ns: String) -> Claim {
        Claim::from_request(ns, self)
    }
}

/// A stored, hash-chained claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub ns: String,
    pub domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    pub assertion: ClaimAssertion,
    pub premises: Vec<PremiseRef>,
    pub consequences: Vec<Consequence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ClaimScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
    pub ts: DateTime<Utc>,
    pub commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_commit: Option<String>,
    pub commit_seq: u64,
}

impl Claim {
    pub fn from_request(ns: String, req: ClaimRequest) -> Self {
        let id = Ulid::new().to_string();
        let ts = Utc::now();
        let seed = format!("{}:{}:{}:{}", &ns, &id, &req.assertion.predicate, ts.to_rfc3339());
        let commit = blake3_hex(seed.as_bytes());
        Self {
            id,
            ns,
            domain: req.domain,
            template: req.template,
            assertion: req.assertion,
            premises: req.premises,
            consequences: req.consequences,
            scope: req.scope,
            cause: req.cause,
            ts,
            commit,
            prev_commit: None,
            commit_seq: 0,
        }
    }
}
