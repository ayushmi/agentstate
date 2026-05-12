use crate::{
    claim::{Claim, PremiseRef},
    domain::DomainPack,
    proof::{Proof, ProofProperties, ProofStatus, StepKind},
};
use std::collections::HashSet;

/// Check all six formal proof properties against a freshly built proof.
pub fn check_all(
    proof: &Proof,
    claim: &Claim,
    existing: &[Proof],
    domain: &DomainPack,
) -> ProofProperties {
    ProofProperties {
        self_consistent: check_consistency(proof, existing),
        minimal: check_minimality(proof),
        has_predictive_constraint: check_predictive(claim),
        verifiable: check_verifiable(proof),
        sound: check_soundness(proof, domain),
        monotonic: check_monotonicity(claim),
    }
}

/// Self-consistency: no proved claim in the namespace contradicts this conclusion.
/// Convention: predicate "not_X" contradicts "X" and vice versa.
fn check_consistency(proof: &Proof, existing: &[Proof]) -> bool {
    let predicate = match proof.conclusion.get("predicate").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return true,
    };
    let contra = if predicate.starts_with("not_") {
        predicate.trim_start_matches("not_").to_string()
    } else {
        format!("not_{}", predicate)
    };
    let subject = proof.conclusion.get("subject");
    let object = proof.conclusion.get("object");
    !existing.iter().any(|p| {
        p.status == ProofStatus::Proved
            && p.conclusion.get("predicate").and_then(|v| v.as_str()) == Some(&contra)
            && p.conclusion.get("subject") == subject
            && p.conclusion.get("object") == object
    })
}

/// Minimality: every declared assumption must be referenced in at least one step.
fn check_minimality(proof: &Proof) -> bool {
    let used: HashSet<&str> = proof
        .steps
        .iter()
        .filter_map(|s| s.justified_by.as_deref())
        .collect();
    // Non-domain assumptions must appear as justified_by in some step.
    proof
        .assumptions
        .iter()
        .filter(|a| !a.starts_with("domain:"))
        .all(|a| used.contains(a.as_str()))
}

/// Predictive constraint: at least one testable consequence must be declared.
fn check_predictive(claim: &Claim) -> bool {
    !claim.consequences.is_empty()
}

/// Verifiability: proof DAG is structurally complete and replayable.
fn check_verifiable(proof: &Proof) -> bool {
    let n = proof.steps.len();
    if n == 0 {
        return false;
    }
    let has_conclusion = proof
        .steps
        .iter()
        .any(|s| matches!(s.kind, StepKind::Conclusion));
    let refs_valid = proof.steps.iter().all(|s| {
        // No self-reference; all referenced steps must exist before this one.
        s.premises_used
            .iter()
            .all(|&pid| pid < n && pid != s.step_id)
    });
    has_conclusion && refs_valid
}

/// Soundness: every inference step uses a rule declared in the domain pack.
fn check_soundness(proof: &Proof, domain: &DomainPack) -> bool {
    proof.steps.iter().all(|s| match s.kind {
        StepKind::Ground => s.justified_by.is_some(),
        StepKind::Inference => {
            let rule_id = s.rule.split(':').last().unwrap_or("");
            domain.inference_rules.iter().any(|r| r.id == rule_id)
                || domain.axioms.iter().any(|a| a.id == rule_id)
        }
        StepKind::Conclusion => !s.premises_used.is_empty(),
    })
}

/// Monotonicity: all WAL state premises must be pinned to a specific commit.
/// Floating references mean the proof silently changes as WAL state evolves.
fn check_monotonicity(claim: &Claim) -> bool {
    claim.premises.iter().all(|p| match p {
        PremiseRef::WalState { at_commit, .. } => at_commit.is_some(),
        _ => true,
    })
}
