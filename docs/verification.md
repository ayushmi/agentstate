# AgentState Verifiability System

AgentState includes three layers of formal verifiability that let you **prove** your AI agents behave correctly — not just observe them.

| Layer | What it does | When to use |
|-------|-------------|-------------|
| 1. Tamper-evident hash chain | Cryptographically chains every write so WAL tampering is detectable | Always on — zero config |
| 2. Runtime invariant assertions | JSON predicate rules checked before every write; violations return 409 | Set once per namespace, enforced forever |
| 3. Temporal property checking | LTL-style properties verified offline over the full WAL trace | In CI/audit pipelines |

---

## Layer 1: Tamper-Evident Hash Chain

Every commit includes `prev_commit` — the blake3 hash of the previous version of the same object. The hash seed includes the `commit_seq`, `prev_commit`, namespace, id, type, timestamp, and body. Reordering or replacing any WAL record breaks the chain.

**Verify via HTTP:**
```bash
curl http://localhost:8080/admin/namespaces/production/chain-verify \
  -H "Authorization: Bearer $AGENTSTATE_API_KEY"
# → {"ok": true, "namespace": "production", "objects_checked": 142, "breaks": []}
```

If `ok` is false, `breaks` lists every object and sequence number where the chain is broken.

**What it detects:**
- WAL record replacement or reordering
- Snapshot injection (an object whose `prev_commit` doesn't match the WAL predecessor)
- Manual edits to the data directory

---

## Layer 2: Runtime Invariant Assertions

Set a predicate spec on a namespace; every subsequent `PUT` is validated against it before being stored. A violation returns HTTP 409 with a structured error.

### Set an invariant

```bash
curl -X POST http://localhost:8080/admin/namespaces/production/invariants \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $AGENTSTATE_API_KEY" \
  -d '{
    "rules": [
      { "field": "body.status", "required": true, "one_of": ["active", "idle", "stopped"] },
      { "field": "body.score",  "gte": 0, "lte": 1 },
      { "field": "tags.env",    "required": true }
    ]
  }'
```

### Get the current invariant

```bash
curl http://localhost:8080/admin/namespaces/production/invariants \
  -H "Authorization: Bearer $AGENTSTATE_API_KEY"
```

### Violation response

When a write violates the invariant:
```json
HTTP 409 Conflict
{
  "error": "invariant_violation",
  "violations": [
    "field 'body.status' must be one of [\"active\",\"idle\",\"stopped\"] but got \"unknown\"",
    "field 'body.score' must be <= 1 but got 1.5"
  ]
}
```

### Predicate DSL reference

| Key | Type | Meaning |
|-----|------|---------|
| `field` | string | Field path: `body.<key>` or `tags.<key>` (dot-separated for nested) |
| `required` | bool | Field must be present and non-null |
| `type` | string | JSON type: `string`, `number`, `bool`, `array`, `object` |
| `eq` | any | Value must equal this exactly |
| `one_of` | array | Value must be one of the listed values |
| `gte`, `lte`, `gt`, `lt` | number | Numeric range checks |
| `regex` | string | String must match pattern (literal + `^`/`$` anchors) |

### Python SDK

```python
from agentstate import AgentStateClient

client = AgentStateClient("http://localhost:8080", "production", api_key="...")

# Set invariant
client.set_invariant("production", [
    {"field": "body.status", "required": True, "one_of": ["active", "idle", "stopped"]},
    {"field": "body.score",  "gte": 0, "lte": 1},
])

# Retrieve invariant
spec = client.get_invariant("production")
print(spec)

# Write that violates it → raises HTTPError (409)
try:
    client.create_agent("agent", {"status": "unknown", "score": 2.0})
except Exception as e:
    print(e)  # invariant_violation: field 'body.status' must be one of ...
```

### Invariants are durable

Invariant specs are persisted as `InvariantSet` records in the WAL. If the server restarts, they are replayed and re-enforced automatically — no re-configuration needed.

---

## Layer 3: Temporal Property Checking

Verify LTL-style properties over the full WAL execution trace. This is an **offline** operation: point it at a WAL directory and property files, and it outputs a structured JSON report.

### CLI usage

```bash
agentstate verify \
  --dir /data/wal \
  --ns production \
  --property props/no_unknown_status.ltl.json \
  --property props/score_eventually_high.ltl.json \
  --output report.json \
  --fail-on-violation
```

Exit code 0 if all properties pass, 1 if any fail (useful in CI).

### Property file format (`.ltl.json`)

```json
{
  "name": "status_never_unknown",
  "kind": "safety",
  "description": "body.status must never equal 'unknown'",
  "forall": { "type": "agent" },
  "always": {
    "not": { "field": "body.status", "eq": "unknown" }
  }
}
```

**Temporal operators:**

| Operator | JSON key | Meaning |
|----------|----------|---------|
| Always | `"always": <pred>` | pred holds at every version of the object |
| Eventually | `"eventually": <pred>` | pred holds at some version |
| Leads-to | `"leads_to": {"if": <pred_a>, "then": {"eventually": <pred_b>}}` | whenever A holds, B must follow |
| Until | `"until": {"hold": <pred_a>, "until": <pred_b>}` | A holds continuously until B |
| Not | `"not": <formula>` | negation |
| And/Or | `"and": [...]` / `"or": [...]` | conjunction/disjunction |

**Selector (`forall`):**
```json
{ "type": "agent" }          // only check objects of this type
{ "ns": "production" }       // only this namespace
{}                            // all objects (default)
```

### Report format

```json
{
  "generated_at": "2026-05-12T10:00:00Z",
  "wal_dir": "/data/wal",
  "namespace": "production",
  "properties_checked": 2,
  "passed": 1,
  "failed": 1,
  "results": [
    {
      "property": "status_never_unknown",
      "kind": "safety",
      "passed": false,
      "violations": [
        {
          "object_id": "01JX...",
          "namespace": "production",
          "commit_seq": 14,
          "ts": "2026-05-11T08:42:00Z",
          "counterexample": { "status": "unknown", "score": 0.8 }
        }
      ]
    }
  ]
}
```

### Use in CI

```yaml
# .github/workflows/verify.yml
- name: Verify agent behavioral properties
  run: |
    agentstate verify \
      --dir $DATA_DIR \
      --ns $NAMESPACE \
      --property props/safety.ltl.json \
      --output verify-report.json \
      --fail-on-violation
- uses: actions/upload-artifact@v4
  with:
    name: verify-report
    path: verify-report.json
```

---

## EU AI Act Mapping

| Article | Requirement | AgentState mechanism |
|---------|------------|---------------------|
| Art. 9 | Risk management system with continuous monitoring | Layer 2 invariants block non-compliant writes in real-time |
| Art. 13 | Transparency and traceability | `cause` field + `prev_commit` hash chain |
| Art. 72 | Post-market monitoring, logging | Layer 3 temporal checking over full WAL trace; `chain-verify` endpoint |

The WAL is the audit log. The hash chain makes it tamper-evident. The temporal properties are the formal behavioral specification.
