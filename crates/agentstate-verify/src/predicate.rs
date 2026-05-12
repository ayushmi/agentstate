/// State predicate evaluator — reuses the same DSL as the Layer 2 invariant checker.
/// Returns true if the predicate holds for the given object's JSON representation.

use agentstate_core::Object;
use serde_json::Value;

/// Evaluate a state predicate against an object.
/// Predicate is a JSON object; the same syntax as the invariant `rules` entries but without
/// the surrounding array — a single rule object here.
///
/// Examples:
///   `{ "field": "body.status", "eq": "active" }`
///   `{ "not": { "field": "body.status", "eq": "unknown" } }`
///   `{ "and": [ { "field": "body.score", "gte": 0 }, { "field": "body.score", "lte": 1 } ] }`
pub fn eval(pred: &Value, obj: &Object) -> bool {
    // Logical operators
    if let Some(inner) = pred.get("not") {
        return !eval(inner, obj);
    }
    if let Some(list) = pred.get("and").and_then(|v| v.as_array()) {
        return list.iter().all(|p| eval(p, obj));
    }
    if let Some(list) = pred.get("or").and_then(|v| v.as_array()) {
        return list.iter().any(|p| eval(p, obj));
    }

    // Field predicate
    let field = match pred.get("field").and_then(|f| f.as_str()) {
        Some(f) => f,
        None => return true, // no field constraint = vacuously true
    };

    let val = resolve_field(field, obj);

    // required
    if pred.get("required").and_then(|v| v.as_bool()).unwrap_or(false) {
        if val.is_none() || val == Some(Value::Null) {
            return false;
        }
    }

    let v = match val {
        Some(v) if !v.is_null() => v,
        _ => return true, // absent, non-required field — predicate doesn't constrain
    };

    // type
    if let Some(expected_type) = pred.get("type").and_then(|t| t.as_str()) {
        let ok = match expected_type {
            "string" => v.is_string(),
            "number" => v.is_number(),
            "bool" | "boolean" => v.is_boolean(),
            "array" => v.is_array(),
            "object" => v.is_object(),
            _ => true,
        };
        if !ok {
            return false;
        }
    }

    // eq
    if let Some(eq_val) = pred.get("eq") {
        if &v != eq_val {
            return false;
        }
    }

    // one_of
    if let Some(choices) = pred.get("one_of").and_then(|a| a.as_array()) {
        if !choices.contains(&v) {
            return false;
        }
    }

    // numeric range
    if let Some(n) = v.as_f64() {
        if let Some(gte) = pred.get("gte").and_then(|x| x.as_f64()) {
            if n < gte {
                return false;
            }
        }
        if let Some(lte) = pred.get("lte").and_then(|x| x.as_f64()) {
            if n > lte {
                return false;
            }
        }
        if let Some(gt) = pred.get("gt").and_then(|x| x.as_f64()) {
            if n <= gt {
                return false;
            }
        }
        if let Some(lt) = pred.get("lt").and_then(|x| x.as_f64()) {
            if n >= lt {
                return false;
            }
        }
    }

    true
}

fn resolve_field(field: &str, obj: &Object) -> Option<Value> {
    if let Some(rest) = field.strip_prefix("body.") {
        let pointer = format!("/{}", rest.replace('.', "/"));
        obj.body.pointer(&pointer).cloned()
    } else if let Some(key) = field.strip_prefix("tags.") {
        obj.tags.0.get(key).map(|s| Value::String(s.clone()))
    } else {
        None
    }
}
