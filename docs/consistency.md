# Consistency & Semantics (MVP)

- Per-namespace snapshot isolation (MVP approximated via last-write-wins with MVCC versions in memory).
- Reads: linear within a single process; no cross-process guarantees.
- Writes: idempotency not yet enforced; clients should retry safely.
- Watches: at-least-once delivery; resume tokens not yet implemented.
- Time-travel: read at or before `ts`; bounded by in-memory retention.

Planned:
- WAL + Raft for CP per-namespace
- Cross-namespace default eventual; optional two-phase commit
- Durable time-travel window; PITR

