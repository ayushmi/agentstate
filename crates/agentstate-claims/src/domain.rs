use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axiom {
    pub id: String,
    pub statement: String,
}

/// An inference rule: given these premise roles are satisfied, this predicate is derived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRule {
    pub id: String,
    /// Premise roles required for this rule to fire.
    pub premises: Vec<String>,
    /// Predicate this rule derives when all premises are present.
    pub conclusion: String,
}

/// A named claim pattern within a domain — defines what evidence is required
/// and what inference chain produces the proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimTemplate {
    pub id: String,
    pub description: String,
    pub required_premises: Vec<String>,
    #[serde(default)]
    pub optional_premises: Vec<String>,
    pub conclusion_predicate: String,
    #[serde(default)]
    pub inference_chain: Vec<String>,
}

/// A constraint that must hold across proved claims in a namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyConstraint {
    pub id: String,
    pub description: String,
    /// If a claim with this predicate is already proved for the same subject+object,
    /// the new claim is self-inconsistent.
    pub contradicts_predicate: String,
}

/// A domain pack — the complete formalization of a real-world domain.
/// Compiled into the binary via include_str!; community packs loaded at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPack {
    pub domain: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub axioms: Vec<Axiom>,
    #[serde(default)]
    pub inference_rules: Vec<InferenceRule>,
    #[serde(default)]
    pub claim_templates: Vec<ClaimTemplate>,
    #[serde(default)]
    pub consistency_constraints: Vec<ConsistencyConstraint>,
}

/// In-process registry of loaded domain packs.
pub struct DomainRegistry {
    packs: HashMap<String, DomainPack>,
}

impl Default for DomainRegistry {
    fn default() -> Self {
        let mut r = Self {
            packs: HashMap::new(),
        };
        r.load_builtins();
        r
    }
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn load_builtins(&mut self) {
        // Domain packs are compiled into the binary for zero-dependency deployment.
        const BUILTINS: &[(&str, &str)] = &[
            (
                "healthcare/v1",
                include_str!("../../../domains/healthcare/v1/manifest.json"),
            ),
            (
                "finance/v1",
                include_str!("../../../domains/finance/v1/manifest.json"),
            ),
            (
                "tax/v1",
                include_str!("../../../domains/tax/v1/manifest.json"),
            ),
            (
                "legal/v1",
                include_str!("../../../domains/legal/v1/manifest.json"),
            ),
        ];
        for (key, json) in BUILTINS {
            match serde_json::from_str::<DomainPack>(json) {
                Ok(pack) => {
                    self.packs.insert(key.to_string(), pack);
                }
                Err(e) => {
                    eprintln!("warn: failed to load built-in domain pack {}: {}", key, e);
                }
            }
        }
    }

    /// Register a community-contributed or runtime-loaded domain pack.
    pub fn register(&mut self, pack: DomainPack) -> String {
        let key = format!("{}/{}", pack.domain, pack.version);
        self.packs.insert(key.clone(), pack);
        key
    }

    /// Load a domain pack from a JSON string (for community packs loaded from disk).
    pub fn load_from_json(&mut self, json: &str) -> Result<String, serde_json::Error> {
        let pack: DomainPack = serde_json::from_str(json)?;
        Ok(self.register(pack))
    }

    /// Look up a domain pack by "domain/version" key, e.g. "healthcare/v1".
    pub fn get(&self, domain_version: &str) -> Option<&DomainPack> {
        self.packs.get(domain_version)
    }

    pub fn list(&self) -> Vec<&DomainPack> {
        self.packs.values().collect()
    }
}
