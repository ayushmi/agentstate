# AgentState Roadmap (User-First)

This roadmap prioritizes getting AgentState into users’ hands quickly, then hardens for scale and reliability. Items are grouped by dependency so teams can parallelize safely.

## Principles
- Self-serve first: frictionless local try, simple hosted path, safe persistence.
- Clear pricing/on-ramp: free tier + predictable paid ramp via gateway metering.
- Production-readiness: backups, restore, observability, and perf baselines.
- Evolve to HA: once adoption is healthy, invest in clustering/consistency.

## Phases (High Level)
- Phase A — Adoption: Playground + gateway + SDKs + adapters + one‑click deploys
- Phase B — Hardening: backups/snapshots, benchmarks, SLOs/alerts
- Phase C — Query/Vector performance: JSONPath engine, materialized indexes, HNSW
- Phase D — HA/Cluster: per‑namespace Raft, cluster ops, chaos testing (multi‑node)
- Phase E — Enterprise/Security: Caps v2, Policy DSL, SSO/SAML/SCIM, quotas

## New Work (not yet issues, supports adoption)
- Playground (static, docs/playground) — DONE (MVP)
- Hosted Gateway (token issuance + metering + Stripe) — Scaffolding added (gateway/)
- One‑click deploys: Render/Fly/Cloud Run templates — TODO

## Issue Matrix

| # | Title | Priority | Area | Impact | Effort | Risk | Dependencies | Next Steps |
|---|-------|----------|------|--------|--------|------|--------------|------------|
| 26 | Docker images for other platforms than linux/arm64/v8 | P0 | infra | Reduce try friction | S/M | Low | — | Buildx multi‑arch to Docker Hub + GHCR; align README tags |
| 24 | Enterprise foundations — SSO/SAML/SCIM + quotas/limits | P1 | compliance | Enterprise adoption | L/XL | Med | Gateway identity/usage | Start with OIDC/SAML in gateway; org model; quotas backed by meter |
| 23 | Benchmarks — YCSB + ANN recall/lat + watch soak | P1 | observability | Perf confidence | M | Med | — | Add YCSB‑style mixes; long watch soak with drop/lag metrics |
| 22 | Jepsen & chaos CI — partitions, disk-full, crash/recovery | P0 | observability | Durability/consistency | L | High | 17 (backup), 14 (ops) | Begin single‑node crash/disk‑full; nightly chaos; expand to cluster later |
| 21 | VS Code extension — state browser, live watch, explain | P2 | sdk | Dev UX | M | Low | 10/12 explain | Read‑only browser + watch; link to Playground |
| 20 | Adapters — LangChain/LangGraph/MCP drop-in memory | P1 | sdk | Framework adoption | M | Low | 10 (query) | Minimal adapters + examples |
| 19 | Go SDK (full parity) + Java SDK (baseline) | P0 | sdk | Backend adoption | M/L | Med | — | REST client parity with Py/TS; 2 examples per SDK |
| 18 | Import/Export — Redis/Postgres/Firestore importers; S3/Parquet export | P1 | import-export | Onboarding/portability | M | Med | 17 (export format) | JSONL/Parquet export first; simple import scripts; docs |
| 17 | Automated snapshots to S3/GCS + lifecycle + PITR to new cluster | P0 | ops | Data safety | M/L | Med | — | Snapshot upload + retention; PITR restore guide + smoke test |
| 16 | Policy DSL — size/budget/PII/region enforced server-side | P1 | policy | Guardrails | L | Med | 10 (path hooks) | Minimal policy spec compiled to handler checks |
| 15 | Caps v2 — EdDSA/JWKS + attenuation/delegation | P0 | security | Secure multitenancy | L/XL | Med/High | Gateway, 24 | JWT/EdDSA with JWKS + rotation; compat with HMAC during migration |
| 14 | Cluster ops — membership, health, rolling upgrades, HA docs | P1 | ops | Operability | L/XL | Med | 13 | Health/readiness, upgrade doc, runbooks |
| 13 | Per-namespace Raft (CP) — leader-only writes, read index | P0 | cluster | HA writes/consistency | XL | High | 10 (state drivers) | RFC + library choice; write path; read index; 3‑node dev cluster |
| 12 | Materialized JSONPath indexes & projections — planner integration | P1 | query | Query perf | L | Med | 10 | Index metadata + lifecycle; planner hits; projection pruning |
| 11 | Vector Index v1 — HNSW (RAM) + lifecycle (READY/BUILDING/STALE) | P0 | vectors | Hybrid retrieval | L | Med | 12 | Embed field, HNSW build/query; async build states; benchmarks |
| 10 | JSONPath Engine v1 — compiled predicates + planner + explain | P0 | query | Expressive + fast queries | L | Med | — | Compile predicates; plan cache; extend /admin/explain; perf tests |
| 9 | Hardening pack — SLO alerts, slow-query hints, restore diff UX | P1 | observability | Trust & ops DX | M | Low | 10/12/17 | SLOs + alerts; slow‑query hints; restore diff script |
| 8 | Delivered — v0.1.0 summary | — | ops | Release hygiene | S | Low | — | Convert to release notes and close |
| 7 | GA Tracker — v0.1.0 | P0 | ops | Release readiness | S | Low | 23/9/17 | Define GA criteria; checklist; tag + SDK publish |

Notes on dependencies:
- 12 depends on 10 (planner/compiled predicates). 11 benefits from 12 (hybrid planner).
- 22 expands after 13/14 for multi‑node chaos, but yields value now for single‑node.
- 24 (SSO/SCIM/quotas) builds on gateway identity + metering; quotas also leverage token claims.
- 17 complements 18 for portability + DR.
- 15 introduces JWT/JWKS while keeping HMAC for transition.

## Prioritized Workstreams (User‑First)

1) Self‑Serve Adoption (Phase A)
- DONE: Static Playground (docs/playground) for local try.
- Gateway MVP (gateway/): token issuance + proxy + metering. Add Stripe Checkout/Portal + usage records; persist usage in Redis/Postgres.
- SDK Expansion: 19 Go SDK (first), Java baseline; 20 Adapters for LangChain/LangGraph/MCP.
- One‑Click Deploys: Render/Fly/Cloud Run templates; README “Deploy in 1 minute”.
- Multi‑arch Images: 26 (in progress); ensure Docker Hub and GHCR parity.

2) Safety & Reliability (Phase B)
- 17 Snapshots to S3/GCS + PITR; nightly smoke restore.
- 23 Benchmarks (YCSB mix; watch soak); publish baseline in README/docs.
- 9 Hardening pack: SLOs, alerts, slow‑query hints, restore diff.

3) Query/Vector Performance (Phase C)
- 10 JSONPath engine v1; 12 Materialized indexes + projections; 11 Vector index v1.

4) HA/Cluster Evolution (Phase D)
- 13 Per‑namespace Raft; 14 Cluster ops docs/practices; expand 22 to cluster chaos.

5) Enterprise/Security (Phase E)
- 15 Caps v2 JWT/JWKS + delegation; 16 Policy DSL; 24 SSO/SAML/SCIM + quotas.

## Milestones & Exit Criteria

- M0 (Adoption MVP): Playground + Gateway (token issuance, metering), Go SDK, Adapters, One‑click deploys; multi‑arch images available.
- M1 (Safe Single‑Node): Snapshots + PITR, Benchmarks published, SLO/alerts, slow‑query hints.
- M2 (Fast Queries): JSONPath engine + materialized indexes + explain; initial vector index.
- M3 (HA Preview): 3‑node Raft per‑namespace; cluster ops docs; chaos CI includes multi‑node.
- M4 (Enterprise Ready): Caps v2, Policy DSL, SSO/SAML/SCIM, org quotas.

---

Last updated: 2025‑08‑25
