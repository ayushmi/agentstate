use crate::{
    checker,
    claim::Claim,
    domain::DomainPack,
    proof::{ConsequenceResult, InferenceStep, Proof, ProofProperties, ProofStatus, StepKind},
};
use agentstate_core::util::blake3_hex;
use chrono::Utc;
use ulid::Ulid;

/// Build a formal proof for a claim against a domain pack and existing namespace proofs.
///
/// The algorithm is template-based forward chaining:
/// 1. One Ground step per declared premise.
/// 2. For each rule in the template's inference_chain, one Inference step.
/// 3. A final Conclusion step referencing all prior steps.
/// 4. Check all six formal properties.
pub fn build_proof(claim: &Claim, domain: &DomainPack, existing_proofs: &[Proof]) -> Proof {
    let mut steps: Vec<InferenceStep> = Vec::new();
    let mut assumptions: Vec<String> = Vec::new();
    assumptions.push(format!("domain:{}/{}", domain.domain, domain.version));

    // — Ground steps (one per premise) —
    for (i, premise) in claim.premises.iter().enumerate() {
        let assumption = premise.assumption_id();
        if !assumptions.contains(&assumption) {
            assumptions.push(assumption.clone());
        }
        steps.push(InferenceStep {
            step_id: i,
            kind: StepKind::Ground,
            rule: format!("ground:{}", premise.role()),
            premises_used: vec![],
            conclusion_predicate: premise.role().to_string(),
            conclusion_desc: format!("Grounded in {}", assumption),
            justified_by: Some(assumption),
        });
    }

    let n_ground = steps.len();

    // — Template validation —
    let template = claim
        .template
        .as_ref()
        .and_then(|t| domain.claim_templates.iter().find(|tmpl| &tmpl.id == t));

    let missing: Vec<String> = if let Some(tmpl) = template {
        let provided: Vec<&str> = claim.premises.iter().map(|p| p.role()).collect();
        tmpl.required_premises
            .iter()
            .filter(|r| !provided.contains(&r.as_str()))
            .cloned()
            .collect()
    } else {
        vec![]
    };

    let (status, refutation_reason) = if !missing.is_empty() {
        (
            ProofStatus::Refuted,
            Some(format!(
                "missing required premises: {}",
                missing.join(", ")
            )),
        )
    } else {
        // — Inference steps —
        if let Some(tmpl) = template {
            let rule_base = n_ground;
            for (i, rule_id) in tmpl.inference_chain.iter().enumerate() {
                if let Some(rule) = domain.inference_rules.iter().find(|r| &r.id == rule_id) {
                    // Bind rule premises to ground steps by role matching.
                    let premises_used: Vec<usize> = (0..n_ground)
                        .filter(|&j| {
                            rule.premises.iter().any(|p| {
                                steps[j]
                                    .rule
                                    .strip_prefix("ground:")
                                    .map(|r| r == p)
                                    .unwrap_or(false)
                            })
                        })
                        .collect();
                    // Fall back to all ground steps if none match by role.
                    let premises_used = if premises_used.is_empty() {
                        (0..n_ground).collect()
                    } else {
                        premises_used
                    };
                    steps.push(InferenceStep {
                        step_id: rule_base + i,
                        kind: StepKind::Inference,
                        rule: format!("{}:{}:{}", domain.domain, domain.version, rule_id),
                        premises_used,
                        conclusion_predicate: rule.conclusion.clone(),
                        conclusion_desc: format!("Rule: {}", rule_id),
                        justified_by: None,
                    });
                }
            }
            // — Conclusion step —
            let all_prev: Vec<usize> = (0..steps.len()).collect();
            steps.push(InferenceStep {
                step_id: steps.len(),
                kind: StepKind::Conclusion,
                rule: format!("{}:{}:{}", domain.domain, domain.version, tmpl.id),
                premises_used: all_prev,
                conclusion_predicate: tmpl.conclusion_predicate.clone(),
                conclusion_desc: format!("QED: {}", claim.assertion.predicate),
                justified_by: None,
            });
        } else {
            // No template — direct assertion, single conclusion step.
            let all_prev: Vec<usize> = (0..n_ground).collect();
            steps.push(InferenceStep {
                step_id: steps.len(),
                kind: StepKind::Conclusion,
                rule: format!("{}:{}:direct", domain.domain, domain.version),
                premises_used: all_prev,
                conclusion_predicate: claim.assertion.predicate.clone(),
                conclusion_desc: format!("Direct: {}", claim.assertion.predicate),
                justified_by: None,
            });
        }
        (ProofStatus::Proved, None)
    };

    let consequences_checked = claim
        .consequences
        .iter()
        .map(|c| ConsequenceResult {
            predicate_desc: serde_json::to_string(&c.predicate).unwrap_or_default(),
            check_after_hours: c.check_after_hours,
            status: "pending".to_string(),
        })
        .collect();

    let conclusion = serde_json::to_value(&claim.assertion).unwrap_or_default();
    let proof_id = Ulid::new().to_string();
    let ts = Utc::now();
    let seed = format!("proof:{}:{}:{}", claim.ns, claim.id, ts.to_rfc3339());
    let commit = blake3_hex(seed.as_bytes());

    let confidence = match &status {
        ProofStatus::Proved => "certain",
        ProofStatus::Refuted => "none",
        _ => "inconclusive",
    }
    .to_string();

    // If required_signers are declared and there are no refutation reasons yet,
    // start in Proving state until all signers co-sign.
    let (status, confidence) = if !claim.required_signers.is_empty()
        && status == ProofStatus::Proved
    {
        (ProofStatus::Proving, "pending_signatures".to_string())
    } else {
        (status, confidence)
    };

    let mut proof = Proof {
        proof_id,
        claim_id: claim.id.clone(),
        ns: claim.ns.clone(),
        domain: claim.domain.clone(),
        status,
        properties: ProofProperties {
            self_consistent: false,
            minimal: false,
            has_predictive_constraint: false,
            verifiable: false,
            sound: false,
            monotonic: false,
            machine_verified: false,
        },
        conclusion,
        confidence,
        steps,
        assumptions,
        consequences_checked,
        challenges: vec![],
        refutation_reason,
        ts,
        commit,
        prev_commit: None,
        commit_seq: 0,
        valid_until: claim.valid_until,
        required_signers: claim.required_signers.clone(),
        lean_certificate: None,
    };

    proof.properties = checker::check_all(&proof, claim, existing_proofs, domain);

    // Self-consistency failure overrides a "proved" status.
    if proof.status == ProofStatus::Proved && !proof.properties.self_consistent {
        proof.status = ProofStatus::Refuted;
        proof.confidence = "none".to_string();
        proof.refutation_reason =
            Some("contradicts an existing proved claim in this namespace".to_string());
    }

    proof
}
