#!/usr/bin/env bash
set -euo pipefail

: "${REPO:?REPO must be set to owner/repo, e.g. export REPO=yourorg/yourrepo}"

# ---------- helpers ----------
json_escape() { jq -Rs . <<< "${1:-}"; }

create_or_update_label() {
  local name="$1" color="$2" desc="$3"
  if gh label view "$name" --repo "$REPO" >/dev/null 2>&1; then
    gh label edit "$name" --color "$color" --description "$desc" --repo "$REPO" >/dev/null
  else
    gh label create "$name" --color "$color" --description "$desc" --repo "$REPO" >/dev/null || true
  fi
}

create_milestone() {
  local title="$1" due="$2" desc="$3"
  # Check existence
  if gh api --method GET "repos/$REPO/milestones?state=all&per_page=100" \
    | jq -e --arg t "$title" '.[] | select(.title==$t)' >/dev/null; then
    # Update description/due_on if needed
    gh api --method PATCH \
      -H "Accept: application/vnd.github+json" \
      "repos/$REPO/milestones/$(gh api "repos/$REPO/milestones?state=all&per_page=100" | jq -r --arg t "$title" '.[] | select(.title==$t) | .number')" \
      -f "title=$title" -f "description=$desc" -f "state=open" -f "due_on=$due" >/dev/null
  else
    gh api --method POST \
      -H "Accept: application/vnd.github+json" \
      "repos/$REPO/milestones" \
      -f "title=$title" -f "state=open" -f "due_on=$due" -f "description=$desc" >/dev/null
  fi
}

issue_create() {
  local title="$1" body="$2" milestone="$3" labels_csv="$4"
  gh issue create --repo "$REPO" --title "$title" --body "$body" --milestone "$milestone" --label "$labels_csv"
}

# ---------- labels ----------
echo "Creating labels…"
create_or_update_label "status:planned"     "cfd3d7" "Planned/not started"
create_or_update_label "status:in-progress" "fbca04" "Actively being worked"
create_or_update_label "status:blocked"     "d93f0b" "Blocked/external dep"
create_or_update_label "status:done"        "0e8a16" "Completed/shipped"
create_or_update_label "priority:P0"        "b60205" "Must-have for milestone"
create_or_update_label "priority:P1"        "d93f0b" "High priority"
create_or_update_label "priority:P2"        "fbca04" "Normal priority"
create_or_update_label "type:feature"       "0e8a16" "New feature/epic"
create_or_update_label "type:enhancement"   "1d76db" "Enhancement/polish"
create_or_update_label "type:bug"           "b60205" "Bug fix/regression"
create_or_update_label "type:doc"           "5319e7" "Docs/site/guides"
create_or_update_label "type:infra"         "0366d6" "Build/CI/Helm/ops"
create_or_update_label "area:storage"       "5319e7" "WAL/snapshots/engine"
create_or_update_label "area:watch"         "0052cc" "Changefeed/watch"
create_or_update_label "area:security"      "6f42c1" "TLS/caps/policy"
create_or_update_label "area:vectors"       "1f883d" "ANN/vector indexes"
create_or_update_label "area:query"         "0e8a16" "JSONPath/indexes/planner"
create_or_update_label "area:ops"           "d93f0b" "Helm/backup/PITR"
create_or_update_label "area:sdk"           "fbca04" "SDKs/CLI/adapters"
create_or_update_label "area:observability" "5319e7" "Metrics/tracing/SLOs"
create_or_update_label "area:import-export" "a2eeef" "Import/export/dump"
create_or_update_label "area:cluster"       "0e8a16" "Raft/multi-node/HA"
create_or_update_label "area:policy"        "6f42c1" "Policy DSL/guards"
create_or_update_label "area:compliance"    "6a737d" "SOC2/SSO/SCIM"

# ---------- milestones (edit dates if you like) ----------
today=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
ms=(
  "v0.1.0 — GA|$(date -u -d '+10 days' +%Y-%m-%dT%H:%M:%SZ)|First GA release from RC: docs, images, chart, verify & dogfood."
  "v0.1.x — Hardening|$(date -u -d '+25 days' +%Y-%m-%dT%H:%M:%SZ)|Bugfixes, polish, SLO alerts, slow-query hints."
  "v0.2.0 — Query & Vector v1|$(date -u -d '+60 days' +%Y-%m-%dT%H:%M:%SZ)|Compiled JSONPath engine + HNSW + index lifecycle."
  "v0.3.0 — Scale & HA|$(date -u -d '+90 days' +%Y-%m-%dT%H:%M:%SZ)|Per-namespace Raft CP, leader-only writes, read index, HA docs."
  "v0.4.0 — Security & Policy v2|$(date -u -d '+120 days' +%Y-%m-%dT%H:%M:%SZ)|EdDSA/JWKS caps, attenuation/delegation, policy DSL."
  "v0.5.0 — Backups, PITR & Import/Export|$(date -u -d '+150 days' +%Y-%m-%dT%H:%M:%SZ)|Automated snapshots to S3/GCS, PITR to new cluster, import/export."
  "v0.6.0 — SDKs & Adapters|$(date -u -d '+180 days' +%Y-%m-%dT%H:%M:%SZ)|Go/Java SDKs, LangChain/LangGraph/MCP adapters, VS Code ext."
  "v0.7.0 — Observability, Jepsen & Benchmarks|$(date -u -d '+210 days' +%Y-%m-%dT%H:%M:%SZ)|Jepsen/chaos CI, YCSB + ANN recall/lat, SLO alert packs."
  "v0.8.0 — Enterprise & Compliance|$(date -u -d '+240 days' +%Y-%m-%dT%H:%M:%SZ)|SSO/SAML/SCIM, quotas, SOC2 program kickoff."
)

echo "Creating milestones…"
for row in "${ms[@]}"; do
  IFS='|' read -r title due desc <<<"$row"
  create_milestone "$title" "$due" "$desc"
done

# ---------- issues ----------
echo "Creating issues…"

# v0.1.0 — GA tracker
issue_create "GA Tracker — v0.1.0"
"**Scope**  
- Tag \`v0.1.0\` (images, SBOM, cosign), publish Helm, publish SDKs.
- Docs: release notes, README quickstart finalize, dashboard JSON link.
- Dogfood: run \`make verify\` in cluster; fix P0s only.

**Acceptance**
- ✅ Images & chart published; \`helm upgrade --install\` works.
- ✅ \`make verify\` green (soak, k6, restore/diff).
- ✅ Release notes posted; perf numbers added to docs/perf.md.
" "v0.1.0 — GA" "status:planned,priority:P0,type:feature,area:ops"

# v0.2.0 — Query & Vector v1 children
issue_create "JSONPath Engine v1 — compiled predicates + planner + explain"
"**Build**
- Compile =, IN, AND/OR, EXISTS, range ops; plan with index selectivity.
- \`/admin/explain-query\` returns indexes_hit, cost, warnings.

**Acceptance**
- QPS ≥ target; scans minimized on indexed fields.
- Explain output stable & documented.
" "v0.2.0 — Query & Vector v1" "status:planned,priority:P0,type:feature,area:query"

issue_create "Vector Index v1 — HNSW (RAM) + lifecycle (READY/BUILDING/STALE)"
"**Build**
- Per-field registry; background build/rebuild; filtered ANN.
- Metrics: vector_query_seconds{field}, recall estimate.

**Acceptance**
- 95% recall @ p95 ≤ target on 1M vectors/node.
- Lifecycle visible in explain; safe rebuild during writes.
" "v0.2.0 — Query & Vector v1" "status:planned,priority:P0,type:feature,area:vectors"

issue_create "Materialized JSONPath indexes & projections — planner integration"
"**Build**
- Opt-in materialized predicates; cost-based choice.
- Projections reduce payload by ≥80% on large docs.

**Acceptance**
- Bench shows 2–10× speedup vs scan paths.
" "v0.2.0 — Query & Vector v1" "status:planned,priority:P1,type:enhancement,area:query"

# v0.3.0 — Scale & HA
issue_create "Per-namespace Raft (CP) — leader-only writes, read index"
"**Build**
- Integrate openraft; shard by namespace; linearizable reads via read index.
- Leader lease; follower catch-up; snapshot transfer.

**Acceptance**
- Jepsen: read-your-writes, no lost updates under partition.
- Rolling upgrade without write stop > N seconds.
" "v0.3.0 — Scale & HA" "status:planned,priority:P0,type:feature,area:cluster"

issue_create "Cluster ops — membership, health, rolling upgrades, HA docs"
"**Build**
- Join/leave APIs; health endpoints; Helm values for replicas.
- HA runbooks & diagrams.

**Acceptance**
- 3-node cluster survives node loss with no data loss; SLOs met.
" "v0.3.0 — Scale & HA" "status:planned,priority:P1,type:infra,area:ops"

# v0.4.0 — Security & Policy v2
issue_create "Caps v2 — EdDSA/JWKS + attenuation/delegation"
"**Build**
- Public-key tokens (kid, jwks URL); macaroons/biscuit-like attenuation.
- Verify across services; rotate without downtime.

**Acceptance**
- Interop demo: two services verify/attenuate without shared secret.
" "v0.4.0 — Security & Policy v2" "status:planned,priority:P0,type:feature,area:security"

issue_create "Policy DSL — size/budget/PII/region enforced server-side"
"**Build**
- Minimal Rego-like or custom DSL; per-namespace policies.
- Deny/allow with audit logs.

**Acceptance**
- Negative tests blocked with correct 4xx; audit captures decision.
" "v0.4.0 — Security & Policy v2" "status:planned,priority:P1,type:feature,area:policy"

# v0.5.0 — Backups, PITR & Import/Export
issue_create "Automated snapshots to S3/GCS + lifecycle + PITR to new cluster"
"**Build**
- Scheduler; remote store; retention; restore into fresh cluster.
- CLI + docs + metrics.

**Acceptance**
- Disaster drill passes: RPO/RTO documented; integrity verified.
" "v0.5.0 — Backups, PITR & Import/Export" "status:planned,priority:P0,type:feature,area:ops"

issue_create "Import/Export — Redis/Postgres/Firestore importers; S3/Parquet export"
"**Build**
- CLI tools; namespace-level dump/load; dev/prod safety.

**Acceptance**
- Round-trip fidelity > 99.99%; perf documented.
" "v0.5.0 — Backups, PITR & Import/Export" "status:planned,priority:P1,type:feature,area:import-export"

# v0.6.0 — SDKs & Adapters
issue_create "Go SDK (full parity) + Java SDK (baseline)"
"**Build**
- put/get/query/watch, leases, idempotency, caps.
- Examples + CI.

**Acceptance**
- Used in verify matrix; docs complete.
" "v0.6.0 — SDKs & Adapters" "status:planned,priority:P0,type:feature,area:sdk"

issue_create "Adapters — LangChain/LangGraph/MCP drop-in memory"
"**Build**
- Replace Redis/SQLite memories with AgentState.
- Samples + blog.

**Acceptance**
- 10-min adoption demo works; users report ‘it just works’.
" "v0.6.0 — SDKs & Adapters" "status:planned,priority:P1,type:feature,area:sdk"

issue_create "VS Code extension — state browser, live watch, explain"
"**Build**
- Read-only explorer; tail events; explain view.

**Acceptance**
- Install + connect within 2 min; UX docs added.
" "v0.6.0 — SDKs & Adapters" "status:planned,priority:P2,type:enhancement,area:sdk"

# v0.7.0 — Observability, Jepsen & Benchmarks
issue_create "Jepsen & chaos CI — partitions, disk-full, crash/recovery"
"**Build**
- Automated faults; assertions for guarantees.
- Badge + docs.

**Acceptance**
- Green across suite on main before release cut.
" "v0.7.0 — Observability, Jepsen & Benchmarks" "status:planned,priority:P0,type:feature,area:observability"

issue_create "Benchmarks — YCSB + ANN recall/lat + watch soak"
"**Build**
- Repro harness; publish numbers; dashboard panels.

**Acceptance**
- Published whitepaper-style doc with configs & results.
" "v0.7.0 — Observability, Jepsen & Benchmarks" "status:planned,priority:P1,type:feature,area:observability"

# v0.8.0 — Enterprise & Compliance
issue_create "Enterprise foundations — SSO/SAML/SCIM + quotas/limits"
"**Build**
- Auth integration; org/project quotas; admin UI.

**Acceptance**
- Enterprise design-partner onboarded; checklists passed.
" "v0.8.0 — Enterprise & Compliance" "status:planned,priority:P1,type:feature,area:compliance"

# v0.1.x — Hardening
issue_create "Hardening pack — SLO alerts, slow-query hints, restore diff UX"
"**Build**
- Alert rules (watch lag, fsync p95, backlog, 4xx caps).
- \"/admin/explain-query\" hints; restore CLI UX.

**Acceptance**
- On-call pack documented; alerts tested.
" "v0.1.x — Hardening" "status:planned,priority:P1,type:enhancement,area:observability"

# v0.1.0 — Delivered summary (what’s built)
issue_create "Delivered — v0.1.0 summary"
"**Shipped**
- WAL + snapshots + recovery + trim; admin endpoints; CLI restore.
- Watch gRPC+SSE with overflow close + resume tokens; SDK watch with checkpoint.
- Idempotency (persisted), leases + fencing; TLS/mTLS; caps (dual HMAC, size/QPS/region).
- Metrics + tracing + Grafana; Helm chart; CI (images+SBOM+sign, SDK publish); docs & RC plan.

**Follow-ups**
- Trackers in subsequent milestones.
" "v0.1.0 — GA" "status:done,type:doc,area:ops"
