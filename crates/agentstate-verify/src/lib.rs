pub mod ltl;
pub mod predicate;

use agentstate_core::Object;
use agentstate_storage::walbin;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// A property to verify, loaded from a `.ltl.json` file.
///
/// Example:
/// ```json
/// {
///   "name": "status_never_unknown",
///   "kind": "safety",
///   "description": "body.status must never equal 'unknown'",
///   "forall": { "type": "agent" },
///   "always": { "not": { "field": "body.status", "eq": "unknown" } }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: String, // "safety" or "liveness"
    #[serde(default)]
    pub description: String,
    /// Filter: only check objects matching this selector.
    /// Supports { "type": "<type>" } or {} for all.
    #[serde(default)]
    pub forall: Option<Value>,
    /// Temporal formula — one of: always, eventually, leads_to, until, not, and, or
    #[serde(flatten)]
    pub formula: Value,
}

fn default_kind() -> String {
    "safety".into()
}

/// A single violation found during verification.
#[derive(Debug, Serialize, Deserialize)]
pub struct Violation {
    pub object_id: String,
    pub namespace: String,
    pub commit_seq: u64,
    pub ts: String,
    pub counterexample: Value,
}

/// Result for a single property.
#[derive(Debug, Serialize, Deserialize)]
pub struct PropertyResult {
    pub property: String,
    pub kind: String,
    pub passed: bool,
    pub violations: Vec<Violation>,
}

/// Full verification report.
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyReport {
    pub generated_at: String,
    pub wal_dir: String,
    pub namespace: Option<String>,
    pub properties_checked: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<PropertyResult>,
}

/// Load properties from a list of `.ltl.json` file paths.
pub fn load_properties(paths: &[String]) -> anyhow::Result<Vec<Property>> {
    let mut props = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read property file '{}': {}", path, e))?;
        let prop: Property = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("cannot parse property file '{}': {}", path, e))?;
        props.push(prop);
    }
    Ok(props)
}

/// Replay a WAL directory into a version map: (ns, id) -> Vec<Object> sorted by commit_seq.
pub fn replay_wal(
    wal_dir: &str,
    ns_filter: Option<&str>,
) -> HashMap<(String, String), Vec<Object>> {
    let recs = walbin::replay(wal_dir).unwrap_or_default();
    let mut map: HashMap<(String, String), Vec<Object>> = HashMap::new();

    for rec in recs {
        if let walbin::RecBody::Put { ns, obj } = rec {
            if let Some(filter) = ns_filter {
                if ns != filter {
                    continue;
                }
            }
            if let Ok(o) = serde_json::from_value::<Object>(obj) {
                map.entry((o.ns.clone(), o.id.clone())).or_default().push(o);
            }
        }
    }

    // Sort each version list by commit_seq
    for versions in map.values_mut() {
        versions.sort_by_key(|o| o.commit_seq);
    }

    map
}

/// Run verification: replay WAL, check each property against matching objects.
pub fn run(wal_dir: &str, ns_filter: Option<&str>, properties: &[Property]) -> VerifyReport {
    let version_map = replay_wal(wal_dir, ns_filter);
    let mut results = Vec::new();
    let mut passed_count = 0;

    for prop in properties {
        let mut violations = Vec::new();

        for ((ns, _id), versions) in &version_map {
            // Apply forall selector
            if let Some(selector) = &prop.forall {
                if let Some(type_filter) = selector.get("type").and_then(|v| v.as_str()) {
                    if !versions
                        .first()
                        .map(|o| o.r#type == type_filter)
                        .unwrap_or(false)
                    {
                        continue;
                    }
                }
                if let Some(ns_filter_val) = selector.get("ns").and_then(|v| v.as_str()) {
                    if ns != ns_filter_val {
                        continue;
                    }
                }
            }

            // Evaluate the temporal formula over the version sequence
            if let Some(v) = ltl::evaluate(&prop.formula, versions) {
                violations.push(v);
            }
        }

        let passed = violations.is_empty();
        if passed {
            passed_count += 1;
        }

        results.push(PropertyResult {
            property: prop.name.clone(),
            kind: prop.kind.clone(),
            passed,
            violations,
        });
    }

    let failed = results.len() - passed_count;
    VerifyReport {
        generated_at: Utc::now().to_rfc3339(),
        wal_dir: wal_dir.to_string(),
        namespace: ns_filter.map(String::from),
        properties_checked: properties.len(),
        passed: passed_count,
        failed,
        results,
    }
}
