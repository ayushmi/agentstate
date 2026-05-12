Chaos Testing (Phase 1)

Goals
- Exercise single-node durability and availability under faults.
- Automate repeatable nemeses: pause (partition surrogate), crash/recovery, disk-full.
- Run in CI on PRs and nightly without touching production images.

How It Works
- Uses a compose overlay that swaps in a Debian runtime image for the server, adds tmpfs for `/data` (size-limited), and assigns a stable container name `agentstate-chaos`.
- Python runner starts the stack, warms with a steady workload, then injects nemeses while validating:
  - Crash/Recovery: `docker kill` then restart; ensure committed objects are readable after restart.
  - Pause/Unpause: `docker pause` to simulate a stall/partition; workload should back off and recover without data loss.
  - Disk-Full: fill `/data` inside the container to trigger ENOSPC; verify server surfaces errors without crashing, and recovers once space is freed.

Usage (local)
- Build and run:
  - `docker compose -f docker-compose.yml -f AgentStateTesting/chaos/docker-compose.chaos.yml up -d --build`
  - `python AgentStateTesting/chaos/chaos_runner.py`
  - `docker compose -f docker-compose.yml -f AgentStateTesting/chaos/docker-compose.chaos.yml down -v`

CI
- `.github/workflows/chaos-ci.yml` runs the chaos suite on PRs labeled `chaos` and nightly via cron. Artifacts (logs, data manifest) are uploaded on failure.

Notes
- Phase 1 targets single-node. Partition tests for multi-node will be added after clustering (see docs/jepsen.md).

