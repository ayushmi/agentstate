/// LTL-style temporal formula evaluator over an ordered sequence of Object versions.
///
/// Supported operators (top-level keys in the property JSON):
///   always:      pred holds at every version
///   eventually:  pred holds at some version
///   leads_to:    { "if": pred_a, "then": { "eventually": pred_b } }
///   until:       { "hold": pred_a, "until": pred_b }
///   not:         negation of another formula
///   and/or:      conjunction/disjunction of multiple formulas
///
/// State predicates (leaf nodes) are handled by `predicate::eval`.

use crate::{predicate, Violation};
use agentstate_core::Object;
use serde_json::Value;

/// Evaluate a temporal formula over a version sequence.
/// Returns Some(Violation) if the formula is violated, None if it holds.
pub fn evaluate(formula: &Value, versions: &[Object]) -> Option<Violation> {
    if versions.is_empty() {
        return None;
    }

    if let Some(pred) = formula.get("always") {
        return check_always(pred, versions);
    }
    if let Some(pred) = formula.get("eventually") {
        return check_eventually(pred, versions);
    }
    if let Some(leads_to) = formula.get("leads_to") {
        return check_leads_to(leads_to, versions);
    }
    if let Some(until_spec) = formula.get("until") {
        return check_until(until_spec, versions);
    }
    if let Some(inner) = formula.get("not") {
        // "not formula" — violation if inner formula does NOT violate
        return if evaluate(inner, versions).is_none() {
            // inner holds, so "not inner" fails
            let first = &versions[0];
            Some(Violation {
                object_id: first.id.clone(),
                namespace: first.ns.clone(),
                commit_seq: first.commit_seq,
                ts: first.ts.to_rfc3339(),
                counterexample: serde_json::json!({"reason": "negated formula holds unexpectedly"}),
            })
        } else {
            None
        };
    }
    if let Some(list) = formula.get("and").and_then(|v| v.as_array()) {
        for sub in list {
            if let Some(v) = evaluate(sub, versions) {
                return Some(v);
            }
        }
        return None;
    }
    if let Some(list) = formula.get("or").and_then(|v| v.as_array()) {
        // "or" holds if at least one sub-formula holds (no violation)
        let all_violated: Vec<_> = list.iter().filter_map(|s| evaluate(s, versions)).collect();
        return if all_violated.len() == list.len() {
            all_violated.into_iter().next()
        } else {
            None
        };
    }

    // Bare predicate (not wrapped in a temporal operator) — evaluate at each version
    check_always(formula, versions)
}

/// always: pred must hold at every version. Returns the first violation.
fn check_always(pred: &Value, versions: &[Object]) -> Option<Violation> {
    for obj in versions {
        if !predicate::eval(pred, obj) {
            return Some(Violation {
                object_id: obj.id.clone(),
                namespace: obj.ns.clone(),
                commit_seq: obj.commit_seq,
                ts: obj.ts.to_rfc3339(),
                counterexample: serde_json::to_value(&obj.body).unwrap_or(Value::Null),
            });
        }
    }
    None
}

/// eventually: pred must hold at some version. Violation if it never holds.
fn check_eventually(pred: &Value, versions: &[Object]) -> Option<Violation> {
    if versions.iter().any(|o| predicate::eval(pred, o)) {
        return None;
    }
    let last = versions.last().unwrap();
    Some(Violation {
        object_id: last.id.clone(),
        namespace: last.ns.clone(),
        commit_seq: last.commit_seq,
        ts: last.ts.to_rfc3339(),
        counterexample: serde_json::json!({
            "reason": "predicate never became true across all versions",
            "final_body": last.body,
        }),
    })
}

/// leads_to: { "if": pred_a, "then": { "eventually": pred_b } }
/// Whenever pred_a holds at version i, pred_b must hold at some version j >= i.
fn check_leads_to(spec: &Value, versions: &[Object]) -> Option<Violation> {
    let if_pred = spec.get("if")?;
    let then_spec = spec.get("then")?;
    let then_pred = then_spec.get("eventually")?;

    for (i, obj) in versions.iter().enumerate() {
        if predicate::eval(if_pred, obj) {
            let suffix = &versions[i..];
            if !suffix.iter().any(|o| predicate::eval(then_pred, o)) {
                return Some(Violation {
                    object_id: obj.id.clone(),
                    namespace: obj.ns.clone(),
                    commit_seq: obj.commit_seq,
                    ts: obj.ts.to_rfc3339(),
                    counterexample: serde_json::json!({
                        "reason": "leads_to: antecedent held but consequent never followed",
                        "antecedent_body": obj.body,
                    }),
                });
            }
        }
    }
    None
}

/// until: { "hold": pred_a, "until": pred_b }
/// pred_a must hold continuously until pred_b becomes true.
fn check_until(spec: &Value, versions: &[Object]) -> Option<Violation> {
    let hold_pred = spec.get("hold")?;
    let until_pred = spec.get("until")?;

    for obj in versions {
        if predicate::eval(until_pred, obj) {
            return None; // reached the "until" condition; satisfied
        }
        if !predicate::eval(hold_pred, obj) {
            return Some(Violation {
                object_id: obj.id.clone(),
                namespace: obj.ns.clone(),
                commit_seq: obj.commit_seq,
                ts: obj.ts.to_rfc3339(),
                counterexample: serde_json::json!({
                    "reason": "until: hold predicate failed before until predicate was satisfied",
                    "body": obj.body,
                }),
            });
        }
    }
    // pred_b never became true — violation if pred_a was supposed to hold until pred_b
    let last = versions.last().unwrap();
    Some(Violation {
        object_id: last.id.clone(),
        namespace: last.ns.clone(),
        commit_seq: last.commit_seq,
        ts: last.ts.to_rfc3339(),
        counterexample: serde_json::json!({
            "reason": "until: the 'until' predicate never became true",
            "final_body": last.body,
        }),
    })
}
