# v0.1.0 RC Freeze Plan

## 0) Scope lock (freeze rules)

- No new features. Only: correctness, security, crash/dataloss fixes, doc clarifications.
- Anything else → open as v0.1.x/v0.2 backlog.

## 1) Branching & versioning

- Create branch: `release/v0.1.0`.
- Bump versions: server/SDKs/Helm to `0.1.0`.
- Tag RC: `v0.1.0-rc.1` (expect 1–2 RCs max).
- Adopt semver contract (see docs/compatibility.md).

## 2) GA contracts (freeze these)

- APIs: HTTP paths & params, gRPC messages, watch semantics (commit_seq, resume, overflow), cap token fields & errors.
- Operational knobs: env vars and defaults (WAL_*, WATCH_*, TLS_*, CAP_*).
- Metrics names: all exported metric identifiers.
- On-disk layout: manifest.json, wal/, snapshots/ (forward-compatible).

After tag: changes require deprecation notes.

## Quality Gates (must pass to unfreeze)

### A) Correctness & safety

- Durability: acknowledged writes survive crash/restart (kill between write() and fsync) — test in CI.
- Recovery: snapshot + WAL tail reproduces exact last_seq.
- Watch contract: overflow close with resume works (no gaps; monotonic seq).
- Idempotency: same key → same response; mismatched body → 409; after restart still holds.
- Leases & fencing: stale writer rejected 100% of time in race tests.
- TLS/mTLS: enabled path works; rejects invalid client certs when required.
- Caps: size/QPS/region enforced; admin routes require admin verbs+ns.

### B) Observability

- `/metrics` exports all dashboard metrics populated (no N/A tiles).
- Tracing spans include cap.kid, jti, ns (auditable).

### C) Performance (single-node, modest box)

- GET p50 < 1ms, p99 < 10ms (warm).
- PUT p50 < 2ms, p99 < 15ms (batched fsync on).
- Watch emit lag p95 < 200ms under steady writes (no overflow).
- ANN 1M vectors, recall≥95% p95 < 20ms (flag current naive as preview if not met — document).

Publish numbers in docs/perf.md with exact machine & cmd lines.

### D) Security & supply chain

- TLS default on in Helm values (document BYO certs).
- Image minimal (distroless/alpine), non-root, read-only FS.
- cargo-deny (advisory & license) and SBOM (CycloneDX) generated; container scanned (Trivy) and clean.

## Test Matrix (pre-GA)

Topologies
- Linux x86_64 & arm64 (containerized).
- HTTP+TLS on/off, gRPC+TLS on/off.
- gRPC watch & SSE watch.

Scenarios
- Crash/recovery under writes.
- Network flap during watch (forced overflow).
- QPS limiter hit & recover.
- Region mismatch returns 451.
- Snapshot+trim → PITR restore → diff vs live dump (dev mode).

Automate with `make verify` that runs soak, k6, restore/diff, and prints a PASS summary.

## Last tiny deltas to land (freeze‑friendly)

1. watch_clients decrement on disconnect
   - SSE: switch to manual stream (dec on drop).
   - gRPC: Drop guard on stream sender.

2. Tracing fields
   - Add cap.kid, jti, ns to spans for put/get/query/watch.

3. Explain-query warnings
   - Include indexes_state (READY|BUILDING|STALE) and emit a warnings array; no behavior change.

All other items → backlog (0.1.1/0.2).

## Semver & deprecation (see docs/compatibility.md)

- Major (X): may break wire formats or watch semantics.
- Minor (Y): new endpoints/metrics/envs; no breaks.
- Patch (Z): fixes/perf only.
- Deprecations: announce in release notes; keep for 2 minors before removal.

## Release Cut Steps (runbook)

1. Freeze: merge last nits → `release/v0.1.0`.
2. Tag RC: `git tag v0.1.0-rc.1 && git push --tags`.
3. Build: multi-arch images; SBOM; sign (Cosign if used).
4. Publish Helm: charts version `0.1.0-rc.1`.
5. Run `make verify`: soak, k6, PITR restore/diff; capture artifacts.
6. RC bake: dogfood in a real cluster with dashboard.
7. Promote: retag `v0.1.0`; publish release notes.

### Release notes template (CHANGELOG.md)

```
## v0.1.0 (YYYY-MM-DD)

### Features
- Durable WAL+snapshots+recovery; admin snapshot/trim; PITR restore CLI.
- Watch contract: commit_seq, overflow close + resume; SDK watch helpers (Py/TS).

### Security
- TLS/mTLS (HTTP & gRPC); capability tokens (dual-key) enforced on all routes.
- Size/QPS/region enforcement; admin caps.

### Observability
- Prometheus metrics for WAL, watch, snapshots, vectors; Grafana dashboard.

### Docs
- Watch contract, key rotation, live dump & restore, perf numbers.

### Breaking changes
- (None for 0.1.0)

### Known limitations
- Vector index is preview; heavy-cardinality JSONPath selectivity is heuristic.
- watch_clients may lag decrement on abrupt client terminations (SSE); fixed in 0.1.1.
```

