# Domain Pack Governance

AgentState's claim verification system is domain-agnostic by design. The
semantics of what a claim *means* — what inference rules are valid, what
premises are required, what conclusions follow — is encoded in **domain packs**.

This document explains how domain packs are structured, how they are reviewed,
and how the community can contribute new ones.

---

## What is a Domain Pack?

A domain pack is a JSON file that declares:

| Section | Purpose |
|---------|---------|
| `axioms` | Foundational assumptions treated as always true in this domain |
| `inference_rules` | Named causal rules: "given premises A, B → conclusion C" |
| `claim_templates` | Reusable claim patterns: required premises + inference chain |

### Example structure

```json
{
  "domain": "healthcare",
  "version": "v1",
  "description": "Clinical decision support",
  "axioms": [
    { "id": "do_no_harm", "statement": "No intervention should increase expected harm" }
  ],
  "inference_rules": [
    {
      "id": "allergy_clearance",
      "premises": ["allergy_record", "drug_interaction_db"],
      "conclusion": "no_contraindication",
      "description": "No known allergy or interaction found"
    }
  ],
  "claim_templates": [
    {
      "id": "drug_safety",
      "description": "Claim that a drug is safe to prescribe",
      "required_premises": ["allergy_record", "drug_interaction_db", "dosage_guidelines"],
      "inference_chain": ["allergy_clearance", "dosage_check"],
      "conclusion_predicate": "safe_to_prescribe"
    }
  ]
}
```

---

## Domain Pack Registry

The canonical registry is at [`domains/registry.json`](../domains/registry.json).
Each entry lists:

- `id` — unique key, e.g. `"healthcare/v1"`
- `path` — relative path to the manifest file
- `status` — `"stable"`, `"beta"`, or `"experimental"`
- `templates` — list of template IDs for discovery
- `regulated_by` — regulations this domain addresses
- `requires_review` — whether submissions undergo mandatory expert review

---

## Built-in Domain Packs

The following packs ship with AgentState and are loaded automatically at server
startup:

| Domain | Version | Status | Key Templates |
|--------|---------|--------|---------------|
| `healthcare` | v1 | stable | drug_safety, diagnostic_certainty, lab_result_interpretation |
| `finance` | v1 | stable | entity_solvency, credit_risk_assessment, contract_validity |
| `legal` | v1 | stable | contract_obligation_met, regulatory_compliance |
| `tax` | v1 | stable | tax_liability, withholding_obligation |

---

## Registering a Custom Domain Pack at Runtime

You can register a domain pack with a running server without restarting:

```bash
# Via CLI
agentstate domain register --server http://localhost:8080 my-domain.json

# Via HTTP
curl -X POST http://localhost:8080/admin/domains \
  -H "Content-Type: application/json" \
  -d @my-domain.json
```

Runtime-registered packs are persisted in the WAL and survive server restarts.
They are listed alongside built-in packs in `GET /admin/domains`.

---

## Validating a Domain Pack

Before registering, validate your pack locally:

```bash
# Via CLI (no server needed)
agentstate domain validate my-domain.json

# Via server
curl -X POST http://localhost:8080/admin/domains/validate \
  -H "Content-Type: application/json" \
  -d @my-domain.json
```

Validation checks:
1. JSON parses as a valid `DomainPack` struct
2. All `inference_chain` rule IDs in templates exist in `inference_rules`
3. No template has empty `required_premises`
4. No rule has empty `id`, `premises`, or `conclusion`

---

## Testing a Domain Pack

Use the CLI to test your pack locally against a sample claim:

```bash
agentstate domain test \
  --domain my-domain.json \
  --claim sample-claim.json \
  --ns test-namespace
```

This runs the full proof engine in-process and prints the resulting proof JSON.
No server required.

For automated testing in CI, use the Rust test harness:

```rust
use agentstate_claims::testing::*;

#[test]
fn drug_safety_requires_three_premises() {
    let harness = DomainHarness::from_file("domains/healthcare/v1/manifest.json").unwrap();

    // Missing dosage_guidelines — should be refuted
    let result = harness.run(ClaimRequest {
        domain: "healthcare".to_string(),
        template: Some("drug_safety".to_string()),
        assertion: ClaimAssertion { predicate: "safe_to_prescribe".to_string(), .. },
        premises: vec![
            source("pub-1", "allergy_record"),
            source("pub-2", "drug_interaction_db"),
        ],
        ..
    });
    assert_refuted!(result);
    assert!(result.refutation_reason().unwrap().contains("dosage_guidelines"));
}
```

---

## Contributing a New Domain Pack

### Tier 1: Community (no review required)

1. Fork this repository.
2. Create `domains/<your-domain>/v1/manifest.json`.
3. Add an entry to `domains/registry.json` with `"status": "experimental"`.
4. Include at least:
   - 2 inference rules
   - 1 claim template
   - 3 example claims (placed in `domains/<your-domain>/v1/examples/`)
5. Open a PR. The CI will run `agentstate domain validate` on your pack.

### Tier 2: Beta (optional expert review)

Mark your pack `"status": "beta"` and request review from a domain expert by
@-mentioning them in the PR. The reviewer signs off with a comment "LGTM:beta".

### Tier 3: Stable (mandatory review)

To become `"stable"` and ship with AgentState by default, a pack must:

1. Receive approval from ≥2 domain experts (e.g. licensed clinicians for
   healthcare, qualified accountants for tax).
2. Have ≥10 unit tests (using the test harness) all passing.
3. Include a regulatory mapping table (see existing packs for examples).
4. Be reviewed by an AgentState maintainer for structural soundness.

### What makes a good domain pack?

- **Specificity**: Inference rules should encode real causal mechanisms from
  your domain, not generic logical tautologies.
- **Falsifiability**: Every template should declare at least one consequence
  that can be checked in WAL state.
- **Minimality**: Required premises should be exactly the evidence needed —
  no more, no less.
- **Monotonicity**: WAL-state premises should include commit pins where
  temporal consistency matters.

---

## Versioning Policy

- `v1`, `v2`, etc. — backwards-incompatible changes increment the major version.
- Within a version, changes that add new templates or rules are allowed if they
  do not change the semantics of existing templates.
- Removing a template or rule requires a new major version.
- Clients pin to a specific version (e.g. `"domain": "healthcare/v1"`) and are
  never automatically migrated.

---

## Governance Committee

The AgentState domain governance committee oversees Tier 3 approvals and
maintains the built-in registry. Current members and their domains of expertise
are listed in [MAINTAINERS.md](../MAINTAINERS.md).

New committee members are nominated by existing members and accepted by
consensus. The committee aims to include practitioners from each regulated
domain covered by built-in packs.
