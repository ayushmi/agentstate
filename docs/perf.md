# Performance Targets & Measurement (v0.1.0)

Targets (single-node, modest box)
- GET: p50 < 1ms, p99 < 10ms (warm)
- PUT: p50 < 2ms, p99 < 15ms (batched fsync)
- Watch: emit lag p95 < 200ms under steady writes (no overflow)
- ANN: 1M vectors, recall≥95% p95 < 20ms (mark as preview if not met)

Environment
- Instance: record CPU type, cores, RAM, storage (NVMe), kernel.
- Build flags: release mode, Rust version.
- Server: DATA_DIR set; WAL batching defaults.

Workloads
- GET/PUT: k6/http or wrk2 with fixed body size; ramp to steady QPS.
- Watch: 1 writer task + N watchers; measure emit lag histogram.
- ANN: synthetic dataset; report recall@k and latency.

Artifacts
- Export Prometheus metrics snapshots and include in release assets.
- Keep exact commands used to generate numbers.

