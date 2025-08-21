# Jepsen Test Plan (Phase 1)

Focus: per-namespace CP semantics (single-shard), write visibility, watch delivery.

Tests:
- Set: concurrent puts on same `(ns,id)`; verify last-write-wins or conflict policy.
- Read Your Writes: client session sees its own writes.
- Monotonic Reads: no time travel to the future across reads.
- Watch Ordering: watch reflects causal order of writes in a namespace.

Harness:
- Nemeses: network partition, process kill, clock skew (future: disk stalls).
- Workload: YCSB A/C mix on JSON payloads; query-by-tags.

Artifacts:
- Repro harness and public report.

