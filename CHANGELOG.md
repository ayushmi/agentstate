## v0.1.0-rc.1 (YYYY-MM-DD)

### Highlights
- Durable WAL+Snapshot + crash-safe recovery
- Per-namespace commit_seq + time-travel reads
- Watch streams with resume tokens (gRPC/SSE), overflow handling & metrics
- Capability tokens (dual-key, size/QPS/region), TLS/mTLS everywhere
- Idempotency keys & persisted leases (fencing)
- SDKs: Python & TypeScript with watch() resume/backoff/checkpoint
- Helm chart, Grafana dashboard, make verify harness

### Ops
- Admin: /admin/snapshot, /admin/trim-wal, /admin/manifest, /admin/explain-query
- Dev-only: /admin/dump (NDJSON)
- Docs: RC plan, compatibility contract, perf targets, key rotation, Helm upgrade

### Known limitation
- SSE uses manual event-stream for precise client counts (no semantic change).

