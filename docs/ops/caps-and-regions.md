# Capability Tokens & Regions (SRE)

## Claims (enforced)

- `ns`: namespace allowlist (required)
- `verbs`: subset of `["put","get","query","watch","lease","admin"]` (required)
- `exp`: UNIX seconds (required)
- `region`: region pin; request rejected with 451 if mismatch to server `REGION`
- `max_bytes`: hard upper bound for request payloads; 413 if exceeded
- `max_qps`: token-bucket rate; 429 on breach
- Optional: `kid` (header), `jti` (id for audit)

## Error mapping

- 401: missing/bad/expired token
- 403: ns not allowed / verb missing
- 413: payload too large (max_bytes)
- 429: QPS exceeded (max_qps)
- 451: region mismatch (claims.region ≠ server REGION)

## Example caps

Admin
```json
{
  "ns":"admin://global",
  "verbs":["admin"],
  "exp": 1737480000,
  "region":"eu-west-1",
  "max_bytes": 10485760,
  "max_qps": 5,
  "jti":"ops-admin-1"
}
```

Service writer
```json
{
  "ns":"agent://acme.support",
  "verbs":["put","get","query","watch","lease"],
  "exp": 1737480000,
  "region":"eu-west-1",
  "max_bytes": 1048576,
  "max_qps": 200,
  "jti":"svc-support-writer"
}
```

## Operational guidance

- Rotation: follow docs/ops/key-rotation.md ACTIVE/NEXT flow; watch 401/403/429/451 and `watch_drops_total`.
- Region pins: deploy per-region; issue region-specific caps to workloads; prevent cross-region drift by policy.
- Budgeting: start conservative (max_bytes ~1–2MB, max_qps per service SLO); raise with evidence (dashboard).

