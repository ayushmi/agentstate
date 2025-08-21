# Architecture (MVP)

- Compute/storage separation via interfaces; current engine is in-memory MVCC.
- Object model: `{id, ns, type, body, tags, ttl_seconds, parents[], commit, ts}`.
- API surface: HTTP JSON for `put/get/query/watch`; gRPC proto defined.
- Watch: SSE over filtered namespace; at-least-once within process lifetime.
- Time-travel: `GetOptions.at_ts` supported in engine; not yet exposed as API param.

Next milestones:
- Pluggable durable engine (LSM-on-SSD) + WAL
- Raft-sharded namespaces
- Vector index module + filtered ANN
- Leases/TTL sweeper and receipts

