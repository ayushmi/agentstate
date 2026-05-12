# Claim Verification

AgentState's claim verification system provides open-source, formal proof of AI agent behavior. Every claim produces a **proof artifact** — a cryptographically hash-chained, fully inspectable DAG that documents exactly why a statement is true.

This system is designed to be the open-source alternative to proprietary AI verification vendors. Verifiability belongs to humanity.

---

## The Proof Model

Every proof satisfies six formal properties:

| Property | Meaning |
|---|---|
| **Self-consistency** | No contradicting proved claims exist in the namespace |
| **Minimality** | Every declared premise is load-bearing |
| **Predictive constraint** | At least one testable consequence is declared |
| **Verifiability** | The proof DAG is complete and fully replayable |
| **Soundness** | Every inference step is licensed by a declared domain rule |
| **Monotonicity** | All WAL state premises are pinned to a specific commit |

---

## Domain Packs

Domain packs formalize real-world reasoning. AgentState ships four built-in packs:

| Domain | Key | Templates |
|---|---|---|
| Healthcare | `healthcare/v1` | `drug_safety`, `contraindication`, `diagnosis`, `lab_normal` |
| Finance | `finance/v1` | `solvency_claim`, `capital_adequacy_claim`, `aml_clearance_claim`, `collateral_adequacy_claim` |
| Tax | `tax/v1` | `deduction_validity`, `filing_status`, `liability_estimate` |
| Legal | `legal/v1` | `contract_validity`, `jurisdiction_claim`, `limitation_claim`, `regulatory_compliance_claim` |

Community packs can be contributed as JSON manifests following the same schema.

---

## HTTP API

### List domain packs

```
GET /admin/domains
```

```json
[
  {
    "domain": "healthcare",
    "version": "v1",
    "description": "Healthcare domain...",
    "templates": ["drug_safety", "contraindication", "diagnosis", "lab_normal"]
  }
]
```

### Submit a claim

```
POST /admin/namespaces/:ns/claims
```

**Request:**

```json
{
  "domain": "healthcare/v1",
  "template": "drug_safety",
  "assertion": {
    "predicate": "safe_to_prescribe",
    "subject": {"patient_id": "p-001"},
    "object": {"drug": "amoxicillin", "dose_mg": 500}
  },
  "premises": [
    {
      "kind": "source",
      "id": "allergy-db-v3",
      "role": "allergy_clearance"
    },
    {
      "kind": "domain_axiom",
      "axiom_id": "drug_class_membership",
      "role": "drug_class"
    },
    {
      "kind": "wal_state",
      "ns": "production",
      "object_id": "patient-p-001",
      "field": "body.allergy_profile",
      "role": "patient_allergy_profile",
      "at_commit": "a1b2c3d4"
    }
  ],
  "consequences": [
    {
      "predicate": {"field": "body.last_safety_check", "required": true},
      "check_after_hours": 24
    }
  ],
  "cause": {"actor": "prescribing-agent", "note": "auto-safety-check"}
}
```

**Response:**

```json
{
  "claim": {
    "id": "01JXABC...",
    "ns": "production",
    "domain": "healthcare/v1",
    "template": "drug_safety",
    "assertion": { "predicate": "safe_to_prescribe", ... },
    "ts": "2026-05-12T10:00:00Z",
    "commit": "blake3hash..."
  },
  "proof": {
    "proof_id": "01JXDEF...",
    "claim_id": "01JXABC...",
    "status": "proved",
    "properties": {
      "self_consistent": true,
      "minimal": true,
      "has_predictive_constraint": true,
      "verifiable": true,
      "sound": true,
      "monotonic": true
    },
    "confidence": "certain",
    "steps": [
      {"step_id": 0, "kind": "ground", "rule": "ground:allergy_clearance", ...},
      {"step_id": 1, "kind": "ground", "rule": "ground:drug_class", ...},
      {"step_id": 2, "kind": "ground", "rule": "ground:patient_allergy_profile", ...},
      {"step_id": 3, "kind": "inference", "rule": "healthcare/v1:allergy_clearance", ...},
      {"step_id": 4, "kind": "inference", "rule": "healthcare/v1:drug_class_safety", ...},
      {"step_id": 5, "kind": "conclusion", "rule": "healthcare/v1:drug_safety", ...}
    ],
    "commit": "blake3hash..."
  }
}
```

### Get a claim

```
GET /admin/namespaces/:ns/claims/:id
```

### Get a proof

```
GET /admin/namespaces/:ns/claims/:id/proof
```

### List claims in a namespace

```
GET /admin/namespaces/:ns/claims
```

### Challenge a proof

```
POST /admin/namespaces/:ns/claims/:id/challenge
```

```json
{
  "challenged_step": 3,
  "reason": "The allergy database queried was not the authoritative source.",
  "counter_evidence": ["allergy-db-v4", "clinical-record-2026-04"]
}
```

---

## CLI

```bash
# List available domain packs
agentstate domain --server http://localhost:8080

# Submit a claim from a JSON file
agentstate claim submit --server http://localhost:8080 --ns production --file claim.json

# Get the proof for a claim
agentstate claim proof --server http://localhost:8080 --ns production 01JXABC...

# List all claims
agentstate claim list --server http://localhost:8080 --ns production

# Challenge a proof (step 3)
agentstate claim challenge --server http://localhost:8080 --ns production 01JXABC... \
  --reason "allergy source is not authoritative" --step 3 --counter allergy-db-v4
```

---

## Python SDK

```python
from agentstate import AgentStateClient

client = AgentStateClient("http://localhost:8080", "production")

# List domains
domains = client.list_domains()

# Submit a claim
result = client.submit_claim(
    ns="production",
    domain="healthcare/v1",
    template="drug_safety",
    assertion={
        "predicate": "safe_to_prescribe",
        "subject": {"patient_id": "p-001"},
        "object": {"drug": "amoxicillin"},
    },
    premises=[
        {"kind": "source", "id": "allergy-db-v3", "role": "allergy_clearance"},
        {"kind": "domain_axiom", "axiom_id": "drug_class_membership", "role": "drug_class"},
    ],
    consequences=[{"predicate": {"field": "body.last_safety_check", "required": True}}],
)
print(result["proof"]["status"])  # "proved"

# Get proof
proof = client.get_proof(ns="production", claim_id=result["claim"]["id"])

# Challenge
challenge = client.challenge_claim(
    ns="production",
    claim_id=result["claim"]["id"],
    reason="Source is not authoritative",
    challenged_step=3,
    counter_evidence=["allergy-db-v4"],
)
```

---

## TypeScript SDK

```typescript
import { AgentStateClient, ClaimRequest } from 'agentstate';

const client = new AgentStateClient('http://localhost:8080', 'production');

// List domains
const domains = await client.listDomains();

// Submit a claim
const result = await client.submitClaim('production', {
  domain: 'healthcare/v1',
  template: 'drug_safety',
  assertion: {
    predicate: 'safe_to_prescribe',
    subject: { patient_id: 'p-001' },
    object: { drug: 'amoxicillin' },
  },
  premises: [
    { kind: 'source', id: 'allergy-db-v3', role: 'allergy_clearance' },
  ],
  consequences: [
    { predicate: { field: 'body.last_safety_check', required: true } }
  ],
});
console.log(result.proof.status); // "proved"

// Challenge
await client.challengeClaim('production', result.claim.id, 'Not authoritative', {
  challenged_step: 3,
  counter_evidence: ['allergy-db-v4'],
});
```

---

## Go SDK

```go
client := agentstate.NewClient("http://localhost:8080", "production")

// List domains
domains, err := client.ListDomains()

// Submit a claim
result, err := client.SubmitClaim("production", map[string]any{
    "domain":   "healthcare/v1",
    "template": "drug_safety",
    "assertion": map[string]any{
        "predicate": "safe_to_prescribe",
        "subject":   map[string]any{"patient_id": "p-001"},
        "object":    map[string]any{"drug": "amoxicillin"},
    },
    "premises": []map[string]any{
        {"kind": "source", "id": "allergy-db-v3", "role": "allergy_clearance"},
    },
})
fmt.Println(result.Proof["status"]) // "proved"

// Challenge
challenge, err := client.ChallengeClaim(
    "production", result.Claim["id"].(string),
    "Not authoritative", 3, []string{"allergy-db-v4"},
)
```

---

## Writing a Community Domain Pack

Create a JSON file following this schema:

```json
{
  "domain": "your-domain",
  "version": "v1",
  "description": "What this domain covers.",
  "axioms": [
    { "id": "axiom_id", "statement": "Human-readable axiom statement." }
  ],
  "inference_rules": [
    {
      "id": "rule_id",
      "premises": ["required_role_1", "required_role_2"],
      "conclusion": "derived_predicate"
    }
  ],
  "claim_templates": [
    {
      "id": "template_id",
      "description": "What this template proves.",
      "required_premises": ["role_1", "role_2"],
      "optional_premises": ["role_3"],
      "conclusion_predicate": "derived_predicate",
      "inference_chain": ["rule_id"]
    }
  ],
  "consistency_constraints": [
    {
      "id": "constraint_id",
      "description": "What contradiction this prevents.",
      "contradicts_predicate": "negated_predicate"
    }
  ]
}
```

Load it at runtime via `DomainRegistry::load_from_json()` or contribute it to the `domains/` directory.

---

## EU AI Act Compliance

| Requirement | Article | How AgentState Satisfies It |
|---|---|---|
| Risk management system | Art. 9 | Runtime invariant assertions reject unsafe writes before they occur |
| Transparency & traceability | Art. 13 | Hash-chained WAL + full proof DAG with step-by-step reasoning |
| Human oversight | Art. 14 | Challenge protocol allows any party to formally contest any proof |
| Accuracy, robustness | Art. 15 | Soundness + monotonicity properties guarantee pinned, non-floating evidence |
| Fundamental rights impact | Art. 72 | Domain packs formalize sector-specific rights constraints (healthcare, legal) |
