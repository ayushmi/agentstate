/// Test harness utilities for domain pack authors.
///
/// Use these helpers to write unit tests for your domain packs without
/// standing up a server. All evaluation is done in-process.
///
/// # Example
/// ```ignore
/// use agentstate_claims::testing::*;
/// use agentstate_claims::{assert_proved, assert_property, ClaimRequest, ClaimAssertion};
///
/// let harness = DomainHarness::from_file("domains/healthcare/v1/manifest.json").unwrap();
/// let result = harness.run_direct("my_predicate", vec![source("pub-1", "evidence")]);
/// assert_proved!(result);
/// assert_property!(result, sound);
/// ```
use crate::{
    build_proof,
    claim::{Claim, ClaimAssertion, ClaimRequest, PremiseRef},
    domain::{DomainPack, DomainRegistry},
    proof::{Proof, ProofStatus},
};

/// A test harness for evaluating claims against a single domain pack.
pub struct DomainHarness {
    pub pack: DomainPack,
    registry: DomainRegistry,
    existing_proofs: Vec<Proof>,
}

impl DomainHarness {
    /// Create a new harness for the given domain pack.
    pub fn new(pack: DomainPack) -> Self {
        let mut registry = DomainRegistry::new();
        registry.register(pack.clone());
        Self {
            pack,
            registry,
            existing_proofs: Vec::new(),
        }
    }

    /// Load a domain pack from a JSON file path.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path, e))?;
        let pack: DomainPack = serde_json::from_str(&raw)
            .map_err(|e| format!("invalid domain pack JSON in {}: {}", path, e))?;
        Ok(Self::new(pack))
    }

    /// Add an existing proved proof so the engine can check self-consistency.
    pub fn with_proof(mut self, proof: Proof) -> Self {
        self.existing_proofs.push(proof);
        self
    }

    /// Run a ClaimRequest through the proof engine and return the proof.
    pub fn run(&self, req: ClaimRequest) -> ProofResult {
        let claim = req.into_claim("test".to_string());
        let proof = build_proof(&claim, &self.pack, &self.existing_proofs);
        ProofResult { claim, proof }
    }

    /// Run a minimal claim with just a predicate, no template.
    pub fn run_direct(&self, predicate: &str, premises: Vec<PremiseRef>) -> ProofResult {
        let req = ClaimRequest {
            domain: self.pack.domain.clone(),
            template: None,
            assertion: ClaimAssertion {
                predicate: predicate.to_string(),
                subject: None,
                object: None,
                params: None,
            },
            premises,
            consequences: vec![],
            scope: None,
            cause: None,
            valid_until: None,
            required_signers: vec![],
        };
        self.run(req)
    }

    /// Validate the domain pack structure: check all inference_chain references exist.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for tmpl in &self.pack.claim_templates {
            for rule_id in &tmpl.inference_chain {
                if !self.pack.inference_rules.iter().any(|r| &r.id == rule_id) {
                    errors.push(format!(
                        "template '{}': inference_chain references unknown rule '{}'",
                        tmpl.id, rule_id
                    ));
                }
            }
            if tmpl.required_premises.is_empty() {
                errors.push(format!(
                    "template '{}': has no required_premises (a template with no requirements proves nothing)",
                    tmpl.id
                ));
            }
        }
        for rule in &self.pack.inference_rules {
            if rule.premises.is_empty() && rule.conclusion.is_empty() {
                errors.push(format!("rule '{}': has no premises and no conclusion", rule.id));
            }
        }
        errors
    }
}

/// The result of running a claim through the proof engine.
pub struct ProofResult {
    pub claim: Claim,
    pub proof: Proof,
}

impl ProofResult {
    pub fn is_proved(&self) -> bool {
        self.proof.status == ProofStatus::Proved
    }
    pub fn is_refuted(&self) -> bool {
        self.proof.status == ProofStatus::Refuted
    }
    pub fn is_proving(&self) -> bool {
        self.proof.status == ProofStatus::Proving
    }

    pub fn refutation_reason(&self) -> Option<&str> {
        self.proof.refutation_reason.as_deref()
    }

    pub fn step_count(&self) -> usize {
        self.proof.steps.len()
    }

    pub fn all_properties_hold(&self) -> bool {
        let p = &self.proof.properties;
        p.self_consistent && p.minimal && p.has_predictive_constraint
            && p.verifiable && p.sound && p.monotonic
    }

    /// Return a list of property names that failed.
    pub fn failing_properties(&self) -> Vec<&'static str> {
        let p = &self.proof.properties;
        let mut fails = Vec::new();
        if !p.self_consistent { fails.push("self_consistent"); }
        if !p.minimal { fails.push("minimal"); }
        if !p.has_predictive_constraint { fails.push("has_predictive_constraint"); }
        if !p.verifiable { fails.push("verifiable"); }
        if !p.sound { fails.push("sound"); }
        if !p.monotonic { fails.push("monotonic"); }
        fails
    }
}

// ── Premise builder helpers ────────────────────────────────────────────────

/// Build a Source premise ref.
pub fn source(id: &str, role: &str) -> PremiseRef {
    PremiseRef::Source {
        id: id.to_string(),
        role: role.to_string(),
    }
}

/// Build a DomainAxiom premise ref.
pub fn axiom(axiom_id: &str, role: &str) -> PremiseRef {
    PremiseRef::DomainAxiom {
        axiom_id: axiom_id.to_string(),
        role: role.to_string(),
    }
}

/// Build a PriorClaim premise ref.
pub fn prior_claim(claim_id: &str, role: &str) -> PremiseRef {
    PremiseRef::PriorClaim {
        claim_id: claim_id.to_string(),
        role: role.to_string(),
    }
}

/// Build a WalState premise ref (pinned at a commit for monotonicity).
pub fn wal_state(ns: &str, object_id: &str, field: &str, role: &str, at_commit: Option<&str>) -> PremiseRef {
    PremiseRef::WalState {
        ns: ns.to_string(),
        object_id: object_id.to_string(),
        field: field.to_string(),
        role: role.to_string(),
        at_commit: at_commit.map(String::from),
    }
}

// ── Assertion macros ───────────────────────────────────────────────────────

/// Assert that a ProofResult is proved. Prints reason on failure.
#[macro_export]
macro_rules! assert_proved {
    ($result:expr) => {
        assert!(
            $result.is_proved(),
            "expected proof to be Proved, got {:?}. Reason: {:?}",
            $result.proof.status,
            $result.refutation_reason()
        );
    };
}

/// Assert that a ProofResult is refuted.
#[macro_export]
macro_rules! assert_refuted {
    ($result:expr) => {
        assert!(
            $result.is_refuted(),
            "expected proof to be Refuted, got {:?}",
            $result.proof.status
        );
    };
}

/// Assert that a specific proof property holds.
/// Property names: self_consistent, minimal, has_predictive_constraint, verifiable, sound, monotonic
#[macro_export]
macro_rules! assert_property {
    ($result:expr, self_consistent) => {
        assert!(
            $result.proof.properties.self_consistent,
            "proof property 'self_consistent' did not hold"
        );
    };
    ($result:expr, minimal) => {
        assert!(
            $result.proof.properties.minimal,
            "proof property 'minimal' did not hold"
        );
    };
    ($result:expr, has_predictive_constraint) => {
        assert!(
            $result.proof.properties.has_predictive_constraint,
            "proof property 'has_predictive_constraint' did not hold"
        );
    };
    ($result:expr, verifiable) => {
        assert!(
            $result.proof.properties.verifiable,
            "proof property 'verifiable' did not hold"
        );
    };
    ($result:expr, sound) => {
        assert!(
            $result.proof.properties.sound,
            "proof property 'sound' did not hold"
        );
    };
    ($result:expr, monotonic) => {
        assert!(
            $result.proof.properties.monotonic,
            "proof property 'monotonic' did not hold"
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pack() -> DomainPack {
        serde_json::from_str(r#"{
            "domain": "test/v1",
            "version": "1.0.0",
            "description": "Test domain for harness unit tests",
            "axioms": [],
            "inference_rules": [
                {
                    "id": "combine",
                    "premises": ["input_a", "input_b"],
                    "conclusion": "combined_result",
                    "description": "Combines two inputs into a result"
                }
            ],
            "claim_templates": [
                {
                    "id": "combine_claim",
                    "description": "Claim that requires two inputs",
                    "required_premises": ["input_a", "input_b"],
                    "inference_chain": ["combine"],
                    "conclusion_predicate": "combined_result"
                }
            ]
        }"#).unwrap()
    }

    #[test]
    fn harness_validates_pack() {
        let harness = DomainHarness::new(sample_pack());
        let errors = harness.validate();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    #[test]
    fn harness_proves_valid_claim() {
        let harness = DomainHarness::new(sample_pack());
        let req = ClaimRequest {
            domain: "test/v1".to_string(),
            template: Some("combine_claim".to_string()),
            assertion: ClaimAssertion {
                predicate: "combined_result".to_string(),
                subject: None,
                object: None,
                params: None,
            },
            premises: vec![
                source("src-1", "input_a"),
                source("src-2", "input_b"),
            ],
            consequences: vec![],
            scope: None,
            cause: None,
            valid_until: None,
            required_signers: vec![],
        };
        let result = harness.run(req);
        assert_proved!(result);
        assert!(result.proof.properties.sound);
        assert!(result.proof.properties.verifiable);
    }

    #[test]
    fn harness_refutes_missing_premise() {
        let harness = DomainHarness::new(sample_pack());
        let req = ClaimRequest {
            domain: "test/v1".to_string(),
            template: Some("combine_claim".to_string()),
            assertion: ClaimAssertion {
                predicate: "combined_result".to_string(),
                subject: None,
                object: None,
                params: None,
            },
            premises: vec![
                source("src-1", "input_a"),
                // missing input_b
            ],
            consequences: vec![],
            scope: None,
            cause: None,
            valid_until: None,
            required_signers: vec![],
        };
        let result = harness.run(req);
        assert_refuted!(result);
        assert!(result.refutation_reason().unwrap().contains("input_b"));
    }

    #[test]
    fn direct_run_proves_without_template() {
        let harness = DomainHarness::new(sample_pack());
        let result = harness.run_direct("my_predicate", vec![source("src-1", "evidence")]);
        assert_proved!(result);
    }
}
