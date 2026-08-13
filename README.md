

# AgentState v1.2.0
**Persistent, formally verifiable state management for AI agents**

[![Docker Build](https://img.shields.io/badge/docker-ready-blue)](#docker-deployment)
[![API Status](https://img.shields.io/badge/api-stable-green)](#api-reference)
[![Load Tested](https://img.shields.io/badge/load%20tested-1400%20ops%2Fs-brightgreen)](#performance)
[![Formally Verifiable](https://img.shields.io/badge/formally-verifiable-purple)](#-formal-verifiability)
[![Claim Verification](https://img.shields.io/badge/claim-verification-orange)](#-claim-verification)
[![Python SDK](https://img.shields.io/badge/python-pypi-blue)](https://pypi.org/project/agentstate/)
[![Node.js SDK](https://img.shields.io/badge/nodejs-npm-green)](https://www.npmjs.com/package/agentstate)

AgentState gives AI agents a persistent, queryable home for their state — with cryptographic behavioral proof built in from the ground up. Think Firebase for AI agents, with the auditability of a formal system.

**What makes it different:**
- Every write is hash-chained — tampering is cryptographically detectable
- Runtime invariants reject bad state before it's stored
- Temporal (LTL) properties checked over the full WAL history
- Claims your agents make can be formally proved, not just logged

---

## Features

- **Real-time state** — subscribe to agent state changes over SSE or gRPC
- **Rich querying** — filter by tags, JSONPath, time-travel to past states
- **Crash-safe persistence** — WAL + snapshots, 1,400+ ops/sec
- **Tamper-evident** — blake3 hash chain across every commit
- **Runtime invariants** — reject invalid writes before they happen
- **Temporal verification** — LTL property checking over WAL traces, in CI
- **Claim verification** — formally prove what your agents assert, not just observe it
- **Domain packs** — pre-built formal reasoning for healthcare, finance, tax, legal
- **Challenge protocol** — anyone can formally contest any proof
- **Language agnostic** — HTTP/gRPC + Python, TypeScript, Go SDKs

---

## Quick Start

```bash
docker run -p 8080:8080 ayushmi/agentstate:latest
```

```python
from agentstate import AgentStateClient

client = AgentStateClient("http://localhost:8080", "production")

agent = client.create_agent(
    agent_type="chatbot",
    body={"name": "CustomerBot", "status": "active"},
    tags={"team": "support"},
)
print(agent["id"])

agents = client.query_agents(tags={"team": "support"})
```

```bash
# Python
pip install agentstate

# TypeScript
npm install agentstate

# Go
go get github.com/ayushmi/agentstate/sdk-go/agentstate
```

---

## Formal Verifiability

AgentState is the only open-source AI agent state store with four complementary layers of formal verification.

### Layer 1 — Tamper-Evident Hash Chain

Every write extends a blake3 chain. Each commit includes `prev_commit` + monotonic sequence — reordering or replacing any WAL record breaks the chain.

```bash
curl http://localhost:8080/admin/namespaces/production/chain-verify
# { "ok": true, "objects_checked": 1842, "breaks": [] }
```

### Layer 2 — Runtime Invariant Assertions

Invariant specs are enforced before every write. Violations are rejected with a structured 409.

```bash
curl -X POST http://localhost:8080/admin/namespaces/production/invariants \
  -d '{
    "rules": [
      { "field": "body.status", "one_of": ["active", "idle", "stopped"] },
      { "field": "body.score",  "gte": 0.0, "lte": 1.0 },
      { "field": "body.agent_id", "required": true }
    ]
  }'
```

Supported predicates: `required`, `type`, `eq`, `one_of`, `gte`/`lte`/`gt`/`lt`, `regex`. Specs are WAL-persisted and survive restarts.

```python
client.set_invariant("production", [
    {"field": "body.status", "one_of": ["active", "idle", "stopped"]},
    {"field": "body.score",  "gte": 0.0, "lte": 1.0},
])
```

### Layer 3 — Temporal Property Checking (LTL)

Write formal properties as JSON files and verify them over the full WAL execution trace — offline, in CI, or post-incident.

```json
{
  "name": "active_agents_eventually_idle",
  "kind": "liveness",
  "forall": { "type": "agent" },
  "leads_to": {
    "if":   { "field": "body.status", "eq": "active" },
    "then": { "eventually": { "or": [
      { "field": "body.status", "eq": "idle" },
      { "field": "body.status", "eq": "stopped" }
    ]}}
  }
}
```

```bash
agentstate-cli verify \
  --dir /data/wal --ns production \
  --property props/liveness.ltl.json \
  --fail-on-violation
```

Supported operators: `always`, `eventually`, `leads_to`, `until`, `not`, `and`, `or`.

### Layer 4 — Claim Verification

See the next section.

---

## Claim Verification

**Verifiability should be open-source.** AgentState lets you formally prove what your AI agents assert — not just log it.

Every claim produces a cryptographically hash-chained **proof artifact**: a full DAG with step-by-step reasoning, checked against six formal properties.

### Six formal proof properties

| Property | Guarantee |
|---|---|
| **Self-consistency** | No contradicting proved claims in the namespace |
| **Minimality** | Every declared premise is load-bearing |
| **Predictive constraint** | At least one testable consequence is declared |
| **Verifiability** | The proof DAG is complete and fully replayable |
| **Soundness** | Every inference step licensed by a declared domain rule |
| **Monotonicity** | All WAL state premises pinned to a specific commit |

### Built-in domain packs

Four domains ship compiled into the binary:

| Domain | Key | Templates |
|---|---|---|
| Healthcare | `healthcare/v1` | `drug_safety`, `contraindication`, `diagnosis`, `lab_normal` |
| Finance | `finance/v1` | `solvency_claim`, `capital_adequacy_claim`, `aml_clearance_claim`, `collateral_adequacy_claim` |
| Tax | `tax/v1` | `deduction_validity`, `filing_status`, `liability_estimate` |
| Legal | `legal/v1` | `contract_validity`, `jurisdiction_claim`, `limitation_claim`, `regulatory_compliance_claim` |

The community can contribute additional domains as JSON manifests — see `domains/` and [docs/claim-verification.md](docs/claim-verification.md).

### Submit a claim

```bash
curl -X POST http://localhost:8080/admin/namespaces/production/claims \
  -H "Content-Type: application/json" \
  -d '{
    "domain": "healthcare/v1",
    "template": "drug_safety",
    "assertion": {
      "predicate": "safe_to_prescribe",
      "subject": {"patient_id": "p-001"},
      "object":  {"drug": "amoxicillin", "dose_mg": 500}
    },
    "premises": [
      {"kind": "source",       "id": "allergy-db-v3",          "role": "allergy_clearance"},
      {"kind": "domain_axiom", "axiom_id": "drug_class_membership", "role": "drug_class"},
      {"kind": "wal_state",    "ns": "production", "object_id": "patient-p-001",
       "field": "body.allergy_profile", "role": "patient_allergy_profile", "at_commit": "a1b2c3d4"}
    ],
    "consequences": [
      {"predicate": {"field": "body.last_safety_check", "required": true}, "check_after_hours": 24}
    ]
  }'
```

Response includes the stored claim and its formal proof:

```json
{
  "claim": { "id": "01JXABC...", "commit": "blake3hash...", ... },
  "proof": {
    "proof_id": "01JXDEF...",
    "status": "proved",
    "confidence": "certain",
    "properties": {
      "self_consistent": true, "minimal": true,
      "has_predictive_constraint": true, "verifiable": true,
      "sound": true, "monotonic": true
    },
    "steps": [
      {"step_id": 0, "kind": "ground",     "conclusion_predicate": "allergy_clearance"},
      {"step_id": 1, "kind": "ground",     "conclusion_predicate": "drug_class"},
      {"step_id": 2, "kind": "ground",     "conclusion_predicate": "patient_allergy_profile"},
      {"step_id": 3, "kind": "inference",  "rule": "healthcare/v1:allergy_clearance"},
      {"step_id": 4, "kind": "inference",  "rule": "healthcare/v1:drug_class_safety"},
      {"step_id": 5, "kind": "conclusion", "conclusion_predicate": "safe_to_prescribe"}
    ]
  }
}
```

### Python SDK

```python
result = client.submit_claim(
    ns="production",
    domain="healthcare/v1",
    template="drug_safety",
    assertion={"predicate": "safe_to_prescribe",
                "subject": {"patient_id": "p-001"},
                "object":  {"drug": "amoxicillin"}},
    premises=[
        {"kind": "source", "id": "allergy-db-v3", "role": "allergy_clearance"},
    ],
    consequences=[{"predicate": {"field": "body.last_safety_check", "required": True}}],
)
print(result["proof"]["status"])   # "proved"
print(result["proof"]["properties"])

# Challenge a proof step
client.challenge_claim(
    ns="production",
    claim_id=result["claim"]["id"],
    reason="allergy database version is outdated",
    challenged_step=0,
    counter_evidence=["allergy-db-v4"],
)
```

### TypeScript SDK

```typescript
const result = await client.submitClaim("production", {
  domain: "healthcare/v1",
  template: "drug_safety",
  assertion: { predicate: "safe_to_prescribe",
               subject: { patient_id: "p-001" },
               object:  { drug: "amoxicillin" } },
  premises: [{ kind: "source", id: "allergy-db-v3", role: "allergy_clearance" }],
});
console.log(result.proof.status); // "proved"
```

### Go SDK

```go
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
```

### CLI

```bash
# List domain packs
agentstate domain

# Submit a claim from a JSON file
agentstate claim submit --ns production --file claim.json

# Get the proof
agentstate claim proof --ns production <claim-id>

# Challenge a step
agentstate claim challenge --ns production <claim-id> \
  --reason "source is outdated" --step 0 --counter allergy-db-v4
```

### Writing a community domain pack

```json
{
  "domain": "your-domain",
  "version": "v1",
  "description": "What this domain covers.",
  "axioms": [
    { "id": "axiom_id", "statement": "Axiom statement." }
  ],
  "inference_rules": [
    { "id": "rule_id", "premises": ["role_a", "role_b"], "conclusion": "derived_predicate" }
  ],
  "claim_templates": [
    {
      "id": "template_id",
      "description": "What this template proves.",
      "required_premises": ["role_a", "role_b"],
      "conclusion_predicate": "derived_predicate",
      "inference_chain": ["rule_id"]
    }
  ]
}
```

Add it to `domains/<your-domain>/v1/manifest.json` and open a PR.

---

## API Reference

### Core

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/v1/{ns}/objects` | Create/update agent state |
| `GET` | `/v1/{ns}/objects/{id}` | Get agent (supports `?at=<RFC3339>` time-travel) |
| `POST` | `/v1/{ns}/query` | Query agents by tags/JSONPath |
| `DELETE` | `/v1/{ns}/objects/{id}` | Delete agent |
| `GET` | `/v1/{ns}/watch` | SSE watch stream |
| `POST` | `/v1/{ns}/lease/acquire` | Acquire a distributed lease |
| `POST` | `/v1/{ns}/lease/renew` | Renew lease |
| `POST` | `/v1/{ns}/lease/release` | Release lease |

### Verification

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/admin/namespaces/{ns}/chain-verify` | Verify blake3 hash chain |
| `POST` | `/admin/namespaces/{ns}/invariants` | Set namespace invariant spec |
| `GET` | `/admin/namespaces/{ns}/invariants` | Get current invariant spec |
| `POST` | `/admin/namespaces/{ns}/claims` | Submit a claim for verification |
| `GET` | `/admin/namespaces/{ns}/claims` | List all claims |
| `GET` | `/admin/namespaces/{ns}/claims/{id}` | Get a claim |
| `GET` | `/admin/namespaces/{ns}/claims/{id}/proof` | Get formal proof artifact |
| `POST` | `/admin/namespaces/{ns}/claims/{id}/challenge` | Challenge a proof |
| `GET` | `/admin/domains` | List domain packs |

### Admin / Ops

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `GET` | `/metrics` | Prometheus metrics |
| `POST` | `/admin/snapshot` | Create WAL snapshot |
| `GET` | `/admin/manifest` | WAL manifest |
| `POST` | `/admin/trim-wal` | Trim WAL segments before snapshot |
| `GET` | `/admin/dump` | Dump all objects as NDJSON |

---

## Performance

- **Write throughput**: 1,400+ ops/sec
- **Read throughput**: 170+ queries/sec
- **Average latency**: ~15ms
- **P95 latency**: ~30ms
- **Reliability**: 0% error rate under load

---

## Deployment

### Docker

```bash
# Development
docker run -p 8080:8080 ayushmi/agentstate:latest

# Production (persistent storage)
docker run -d \
  -p 8080:8080 -p 9090:9090 \
  -e DATA_DIR=/data \
  -v agentstate-data:/data \
  --restart unless-stopped \
  ayushmi/agentstate:latest
```

### Docker Compose

```yaml
services:
  agentstate:
    image: ayushmi/agentstate:latest
    ports: ["8080:8080", "9090:9090"]
    environment:
      DATA_DIR: /data
    volumes:
      - agentstate-data:/data
    restart: unless-stopped

volumes:
  agentstate-data:
```

### Build from source

```bash
git clone https://github.com/ayushmi/agentstate.git
cd agentstate
cargo build --release -p agentstate-server
./target/release/agentstate-server
```

---

## EU AI Act Compliance

| Article | Requirement | Satisfied by |
|---|---|---|
| Art. 9 | Risk management system | Runtime invariant assertions reject unsafe writes |
| Art. 13 | Transparency & traceability | Hash chain + full proof DAG with step-by-step reasoning |
| Art. 14 | Human oversight | Challenge protocol — any party can contest any proof |
| Art. 15 | Accuracy, robustness | Soundness + monotonicity properties guarantee pinned evidence |
| Art. 72 | Post-market monitoring | LTL temporal verification in CI, structured claim audit trail |

---

## Documentation

- [Claim Verification](docs/claim-verification.md) — proof model, domain packs, EU AI Act mapping
- [Verification Guide](docs/verification.md) — hash chains, invariants, LTL
- [Roadmap](docs/ROADMAP.md)
- [Examples](examples/)

---

## Contributing

Community domain packs are especially welcome. If you can formalize a domain (agriculture, cybersecurity, clinical trials, education, ...) follow the JSON schema in `domains/` and open a pull request.

For bugs and features, use [GitHub Issues](https://github.com/ayushmi/agentstate/issues).

---

## Our Mission

AgentState is open-source infrastructure built in service of a larger goal: **autonomous AI that can operate the world — physically and digitally — in a way that is causal and verifiable by design.**

We are building a causal world model for mechanism inference. The aim is not AI that predicts what usually happens, but AI that understands *why* things happen — and can therefore act on that understanding with confidence. This powers Actions, our product for autonomous operations: dispatching setpoint changes to PLCs, coordinating human workflows, and executing in digital systems. We operate across industrial environments and biology.

When AI acts in the physical world, the stakes are real. A wrong setpoint in a manufacturing process or a biological system is not a bad recommendation — it can be catastrophic. Verifiability is not a compliance checkbox in this context; it is a hard requirement for deployment.

That is why verifiability is open-source. Trust in high-stakes AI cannot be built on closed systems. It requires infrastructure that anyone can inspect, challenge, and contribute to. AgentState gives teams the primitives to prove — not just assert — that their AI is doing the right thing for the right reasons.

If you are working on autonomous operations, industrial AI, or biological systems and care about getting this right, we would like to hear from you.

---

## License

Apache-2.0
