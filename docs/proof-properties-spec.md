# Formal Specification of the Six Proof Properties

AgentState enforces six formal properties on every proof artifact. A proof is
considered **fully valid** only when all six hold. This document defines each
property precisely, provides the evaluation algorithm, and gives examples of
compliant and non-compliant claims.

---

## Overview

| # | Property | Formal Invariant |
|---|----------|-----------------|
| 1 | Self-consistent | ¬∃ (C, ¬C) both proved in same namespace |
| 2 | Minimal | every declared premise is load-bearing |
| 3 | Predictive constraint | ≥1 testable consequence declared |
| 4 | Verifiable | proof DAG is complete and replayable |
| 5 | Sound | every inference step licensed by a domain rule |
| 6 | Monotonic | all WAL-state premises pinned to a specific commit |

---

## Property 1 — Self-Consistent

**Informal:** No claim and its direct negation can both be in the `proved`
state within the same namespace at the same time.

**Formal definition:**

Let *P(ns)* be the set of proved claims in namespace *ns*.
Let *neg(p)* be the negation of predicate *p*, defined as:

- If *p* = `"X"`, then *neg(p)* = `"not_X"` or `"¬X"`.
- More generally: two predicates *p* and *q* are *contradictory* if
  `q == "not_" + p` or `p == "not_" + q`.

**Self-consistent(C, P(ns))** iff ∀ C' ∈ P(ns), the predicates of C and C'
are not contradictory.

**Evaluation algorithm** (`checker.rs`):

```
fn check_self_consistent(proof, claim, existing_proofs) -> bool {
    let pred = &claim.assertion.predicate;
    let neg = format!("not_{}", pred);
    for ep in existing_proofs {
        if ep.ns == claim.ns
           && ep.status == Proved
           && (ep.conclusion["predicate"] == neg
               || format!("not_{}", ep.conclusion["predicate"]) == pred)
        {
            return false;
        }
    }
    true
}
```

**Example — violation:**
```json
// Existing proved claim in namespace "trading"
{ "assertion": { "predicate": "entity_solvent" } }

// New claim being submitted — will be Refuted (self-consistency check fails)
{ "assertion": { "predicate": "not_entity_solvent" } }
```

---

## Property 2 — Minimal

**Informal:** Every declared premise is actually used in deriving the
conclusion. A proof with surplus premises is not minimal.

**Formal definition:**

Let *G = {g₁, …, gₙ}* be the ground steps of the proof.
The proof is **minimal** iff for every *gᵢ ∈ G*, removing *gᵢ* causes at
least one inference step to lose a required premise, i.e., the remaining ground
steps no longer cover all roles required by the template's `required_premises`.

**Evaluation algorithm:**

```
fn check_minimal(proof, claim, domain) -> bool {
    let template = domain.claim_templates.find(claim.template);
    if template is None { return true; }  // direct claims are trivially minimal

    let required = template.required_premises;
    let provided_roles: Set = claim.premises.map(|p| p.role());

    // Every provided role must appear in required_premises.
    // Surplus roles (not in required) make the proof non-minimal.
    provided_roles.is_subset_of(required)
}
```

**Example — non-minimal claim:**
```json
{
  "template": "drug_safety",
  "premises": [
    { "kind": "source", "id": "pub-1", "role": "allergy_record" },
    { "kind": "source", "id": "pub-2", "role": "drug_interaction_db" },
    { "kind": "source", "id": "pub-9", "role": "unrelated_note" }  // ← surplus
  ]
}
// Proof minimal: false — "unrelated_note" is not a required premise
```

---

## Property 3 — Predictive Constraint

**Informal:** The claim makes at least one testable, falsifiable prediction —
something that must hold in the WAL state as a consequence of accepting the
claim. Claims with no consequences are unfalsifiable and score lower.

**Formal definition:**

**has_predictive_constraint(claim)** iff `|claim.consequences| ≥ 1`.

**Evaluation algorithm:**

```
fn check_has_predictive_constraint(claim) -> bool {
    !claim.consequences.is_empty()
}
```

**Recommended consequence format:**
```json
{
  "consequences": [
    {
      "predicate": { "field": "body.status", "eq": "approved" },
      "check_after_hours": 24
    }
  ]
}
```

The consequence daemon evaluates these predicates against live WAL state
24 hours after submission and marks the consequence `holds` or `violated`.
A violated consequence causes the proof status to become `refuted`.

---

## Property 4 — Verifiable

**Informal:** The proof DAG is complete: every inference step references valid
prior step IDs, and the DAG has exactly one root (the Conclusion step) and at
least one leaf (a Ground step). The proof can be mechanically replayed from its
assumptions to its conclusion without additional context.

**Formal definition:**

Let *S = {s₀, …, sₙ}* be the steps in topological order.
**verifiable(S)** iff:

1. ∀ *sᵢ* with `kind = Inference`: all `premises_used` indices are < *i*.
2. ∃ exactly one *sₙ* with `kind = Conclusion`.
3. ∃ at least one *s₀* with `kind = Ground`.
4. All `assumptions` are non-empty strings.
5. Every Ground step has a non-null `justified_by`.

**Evaluation algorithm:**

```
fn check_verifiable(proof) -> bool {
    let has_ground = proof.steps.iter().any(|s| s.kind == Ground);
    let has_conclusion = proof.steps.iter().filter(|s| s.kind == Conclusion).count() == 1;
    let dag_ok = proof.steps.iter().enumerate().all(|(i, s)| {
        s.kind != Inference
            || s.premises_used.iter().all(|&j| j < i)
    });
    let ground_justified = proof.steps.iter().all(|s| {
        s.kind != Ground || s.justified_by.is_some()
    });
    has_ground && has_conclusion && dag_ok && ground_justified
}
```

---

## Property 5 — Sound

**Informal:** Every Inference step is licensed by a rule declared in the domain
pack — no conclusions are asserted without a declared causal mechanism.

**Formal definition:**

**sound(proof, domain)** iff ∀ *sᵢ* with `kind = Inference`:
the rule identifier `sᵢ.rule` matches a rule in `domain.inference_rules`.

The rule identifier format is `"<domain>/<version>:<rule_id>"`, e.g.
`"healthcare/v1:allergy_clearance"`.

**Evaluation algorithm:**

```
fn check_sound(proof, domain) -> bool {
    proof.steps.iter().all(|s| {
        if s.kind != Inference { return true; }
        let rule_suffix = s.rule.split(':').last().unwrap_or("");
        domain.inference_rules.iter().any(|r| r.id == rule_suffix)
    })
}
```

**Example — unsound proof:**
A direct claim (no template) generates a single Conclusion step with
`rule = "<domain>/v1:direct"`. This is sound if the domain exists. An Inference
step with `rule = "healthcare/v1:unknown_rule"` where `unknown_rule` is not
declared in the domain's `inference_rules` would be unsound.

---

## Property 6 — Monotonic

**Informal:** All WAL-state premises are pinned to a specific commit hash.
"Floating" WAL references (no `at_commit`) violate monotonicity because
replaying the proof at a later time would use different state values.

**Formal definition:**

**monotonic(claim)** iff ∀ *p ∈ claim.premises* with `kind = wal_state`:
`p.at_commit` is present and non-null.

Claims that reference only `source`, `domain_axiom`, or `prior_claim` premises
are trivially monotonic (they have no floating WAL dependencies).

**Evaluation algorithm:**

```
fn check_monotonic(claim) -> bool {
    claim.premises.iter().all(|p| match p {
        WalState { at_commit, .. } => at_commit.is_some(),
        _ => true,
    })
}
```

**Example — non-monotonic claim:**
```json
{
  "premises": [
    {
      "kind": "wal_state",
      "ns": "trading",
      "object_id": "account-42",
      "field": "body.balance",
      "role": "account_balance"
      // missing "at_commit" → non-monotonic
    }
  ]
}
```

**Fix:**
```json
{
  "kind": "wal_state",
  "ns": "trading",
  "object_id": "account-42",
  "field": "body.balance",
  "role": "account_balance",
  "at_commit": "a3f8c2d1"
}
```

---

## Composite Validity

A proof with `status = proved` and all six properties `true` is considered
**fully valid**. The HTTP API exposes the property breakdown in every proof
response under the `properties` field:

```json
{
  "proof_id": "01JX...",
  "status": "proved",
  "confidence": "certain",
  "properties": {
    "self_consistent": true,
    "minimal": true,
    "has_predictive_constraint": true,
    "verifiable": true,
    "sound": true,
    "monotonic": true
  }
}
```

If any property is `false`, the proof is still stored but should be treated as
**provisionally valid** pending correction of the flagged issue.

---

## EU AI Act Mapping

| Property | EU AI Act Article | Requirement |
|----------|-------------------|-------------|
| Self-consistent | Art. 9(1)(a) | No conflicting risk conclusions in same system |
| Predictive constraint | Art. 9(1)(b) | Claims must produce testable outcomes |
| Verifiable | Art. 13(1) | Audit trail must be complete and replayable |
| Sound | Art. 9(4) | Reasoning must be grounded in recognised methods |
| Monotonic | Art. 72 | Records must not change meaning retroactively |
| Minimal | Art. 10(2)(f) | No redundant or misleading evidence |
