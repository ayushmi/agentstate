use crate::model::Object;
use serde_json::Value;

/// Check an object against a namespace invariant spec.
/// Returns Ok(()) if all rules pass, or Err with a list of violation messages.
///
/// Spec format:
/// ```json
/// { "rules": [
///   { "field": "body.status", "type": "string" },
///   { "field": "body.score",  "gte": 0, "lte": 1 },
///   { "field": "body.name",   "required": true },
///   { "field": "tags.env",    "one_of": ["prod", "staging"] },
///   { "field": "body.label",  "regex": "^exact" }
/// ]}
/// ```
///
/// Field paths: `body.<key>` navigates into the body JSON, `tags.<key>` into tags.
pub fn check(spec: &Value, obj: &Object) -> Result<(), Vec<String>> {
    let rules = match spec.get("rules").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => return Ok(()),
    };

    let mut violations = Vec::new();

    for rule in rules {
        let field = match rule.get("field").and_then(|f| f.as_str()) {
            Some(f) => f,
            None => continue,
        };

        let value = resolve_field(field, obj);

        // required
        if rule.get("required").and_then(|v| v.as_bool()).unwrap_or(false) {
            if value.is_none() || value == Some(Value::Null) {
                violations.push(format!("field '{}' is required but missing or null", field));
                continue;
            }
        }

        let val = match &value {
            Some(v) if !v.is_null() => v.clone(),
            _ => continue,
        };

        // type check
        if let Some(expected_type) = rule.get("type").and_then(|t| t.as_str()) {
            let actual_ok = match expected_type {
                "string" => val.is_string(),
                "number" => val.is_number(),
                "bool" | "boolean" => val.is_boolean(),
                "array" => val.is_array(),
                "object" => val.is_object(),
                _ => true,
            };
            if !actual_ok {
                violations.push(format!(
                    "field '{}' must be of type '{}' but got {}",
                    field,
                    expected_type,
                    json_type_name(&val)
                ));
            }
        }

        // eq
        if let Some(eq_val) = rule.get("eq") {
            if &val != eq_val {
                violations.push(format!("field '{}' must equal {}", field, eq_val));
            }
        }

        // one_of
        if let Some(choices) = rule.get("one_of").and_then(|v| v.as_array()) {
            if !choices.contains(&val) {
                violations.push(format!(
                    "field '{}' must be one of {} but got {}",
                    field,
                    serde_json::to_string(choices).unwrap_or_default(),
                    val
                ));
            }
        }

        // numeric range (gte, lte, gt, lt)
        if let Some(n) = val.as_f64() {
            if let Some(gte) = rule.get("gte").and_then(|v| v.as_f64()) {
                if n < gte {
                    violations.push(format!("field '{}' must be >= {} but got {}", field, gte, n));
                }
            }
            if let Some(lte) = rule.get("lte").and_then(|v| v.as_f64()) {
                if n > lte {
                    violations.push(format!("field '{}' must be <= {} but got {}", field, lte, n));
                }
            }
            if let Some(gt) = rule.get("gt").and_then(|v| v.as_f64()) {
                if n <= gt {
                    violations.push(format!("field '{}' must be > {} but got {}", field, gt, n));
                }
            }
            if let Some(lt) = rule.get("lt").and_then(|v| v.as_f64()) {
                if n >= lt {
                    violations.push(format!("field '{}' must be < {} but got {}", field, lt, n));
                }
            }
        }

        // regex (literal patterns with optional ^ / $ anchors)
        if let Some(pattern) = rule.get("regex").and_then(|v| v.as_str()) {
            if let Some(s) = val.as_str() {
                match simple_regex_match(pattern, s) {
                    Ok(false) => violations.push(format!(
                        "field '{}' value '{}' does not match regex '{}'",
                        field, s, pattern
                    )),
                    Err(e) => violations.push(format!("field '{}' regex error: {}", field, e)),
                    Ok(true) => {}
                }
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Resolve a field path to an owned Value.
/// Supports "body.<key.nested>" and "tags.<key>".
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

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Minimal regex: supports ^ anchor, $ anchor, and literal substrings.
/// For complex patterns, returns Err with a helpful message.
fn simple_regex_match(pattern: &str, s: &str) -> Result<bool, String> {
    let anchored_start = pattern.starts_with('^');
    let anchored_end = pattern.ends_with('$');
    let core = pattern.trim_start_matches('^').trim_end_matches('$');

    let has_specials = core
        .chars()
        .any(|c| matches!(c, '.' | '+' | '*' | '[' | '(' | ')' | '?' | '{' | '}' | '|' | '\\'));

    if has_specials {
        return Err(format!(
            "complex regex '{}' not supported; use literal patterns with ^ and $ anchors only",
            pattern
        ));
    }

    let matched = match (anchored_start, anchored_end) {
        (true, true) => s == core,
        (true, false) => s.starts_with(core),
        (false, true) => s.ends_with(core),
        (false, false) => s.contains(core),
    };
    Ok(matched)
}
