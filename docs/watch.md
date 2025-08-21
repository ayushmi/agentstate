# Watch v1.5 (MVP)

- Protocols: gRPC streaming at `:9090` `Watch(WatchRequest)` and HTTP SSE at `/v1/{ns}/watch`.
- Resume tokens: every event includes a `commit` (monotonic per-namespace). Pass `from_commit` in gRPC `WatchRequest` to resume.
- Backpressure: server buffers per-subscriber in-memory; if slow, events will accumulate; clients should resume with token after reconnect.
- Semantics: at-least-once delivery; events are idempotent by `id` and `commit`.

## Contract (v1)

- Delivery: at-least-once.
- Ordering: per-namespace monotonic by `commit_seq` (strictly increasing, never reused).

### Event Shape
- gRPC: message includes `commit` (required).
- SSE: `id: <commit_seq>` and JSON `{ "commit_seq": <u64>, ... }` in `data:`.

### Resuming
- Pass `from_commit=<u64>` (inclusive). Server will resend from that commit.

### Overflow
- gRPC: server closes stream with RESOURCE_EXHAUSTED and message
  `"overflow last_commit=<u64> retry_after_ms=<u32>"`.
- SSE: server emits a final event
  `id:<last_commit>` with `{ "error":"overflow","last_commit":<u64> }` then closes.
- Clients must resume from the indicated `last_commit` with jittered backoff.

### Client Strategy
- Maintain `last_commit` (optionally checkpoint to disk).
- On disconnect or overflow, jittered backoff, then resume.
- Expect duplicates; make consumers idempotent.

### Not Guaranteed
- Exactly-once delivery (use Idempotency-Key).
- Cross-namespace global ordering (only per namespace).

### Prometheus Panels (examples)
- `sum(watch_clients)`
- `rate(watch_events_total[1m])`
- `rate(watch_resumes_total[1m])`
- `max_over_time(watch_backlog_events{ns!=""}[5m])`
- `histogram_quantile(0.95, sum(rate(watch_emit_lag_seconds_bucket[5m])) by (le))`
- `histogram_quantile(0.95, sum(rate(wal_fsync_seconds_bucket[5m])) by (le))`
- `histogram_quantile(0.95, sum(rate(op_duration_seconds_bucket{op="put"}[5m])) by (le))`
