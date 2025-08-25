In-memory Chaos: Bringing Jepsen-Style CI to a Single-Node Server

25th Aug 2025

I’ve been tightening up durability and availability guarantees for AgentState, and wanted a way to catch the “bad day” bugs before users do: crash loops, disk-full, pauses that look like partitions. We don’t have clustering wired yet, but that shouldn’t block us from doing meaningful chaos. This post is a short field report on how we built a non‑intrusive, repeatable chaos CI that runs against a single node and still finds real issues.

Why do chaos before clustering?
- Because single-node durability bugs hurt just as much as distributed ones.
- Because disciplined tests around crash/recovery and ENOSPC shape better storage choices.
- Because the cheapest time to add observability is before you’re juggling quorum and elections.

Constraints
- No changes to the server runtime. Prove we can validate from the outside: public HTTP API, SSE watch, Prometheus `/metrics`.
- Repeatable in CI. No artisanal tmux scripts; it has to run on every PR and nightly.
- Fast feedback. Minutes, not hours.

The smallest possible harness that matters
We added a compose overlay that swaps in a Debian runtime image for the server, mounts a 64 MiB tmpfs at `/data`, and gives the container a stable name. That’s all we needed to:
- Crash: `docker kill` the process, then bring it back up and check durability.
- Pause: `docker pause` for 5 seconds to simulate a stall/partition surrogate; then unpause and watch clients recover.
- Disk-full: fill the tmpfs using `fallocate` (or `dd` fallback) so the filesystem hits ENOSPC quickly and predictably.

Workload, not benchmarks
I didn’t chase microsecond latencies here. The point was correctness under stress. The workload is intentionally simple:
- Writers continually PUT JSON objects with small updates across a handful of keys.
- Readers GET random keys. We record the last intended value per key.
- A watcher opens the SSE endpoint (`/v1/:ns/watch`) and asserts that:
  - commit sequence ids are monotonically increasing globally;
  - for a sample of keys, the last seen body on the watch stream matches what writers intended;
  - the stream doesn’t overflow (server emits an explicit overflow event if it does).

Metrics as tripwires
If a chaos suite can’t tell you “this is worse,” it’s theatre. We scrape `/metrics` and assert three invariants:
- `watch_drops_total{reason="overflow"} == 0` — if this trips, we’re losing events.
- `watch_clients{proto="sse"} >= 1` — sanity check the watcher actually stayed connected during the run.
- `watch_events_total > 0` — the pipeline is processing events.

Disk-full: directing the pain
I used a tmpfs so disk pressure is precise and quick. A too‑generous tmpfs wastes CI minutes; too small makes the run flaky. 64 MiB was a sweet spot for us: fill ~56 MiB, watch PUTs start failing, free space, then assert that writes succeed again. The server already returns errors on write failure and recovers cleanly once space is available, which is all we needed at this stage.

Pause and crash: watching the edges
Pausing the container stalled everything as expected; the SSE client stayed connected after unpause and continued consuming. A hard `SIGKILL` followed by restart required a small reconnect window, but the watcher resumed and caught up without gaps. We specifically validated sequence monotonicity and last‑value agreement after both nemeses.

What we didn’t change (on purpose)
- We didn’t add special “test hooks” to the server. If chaos needs a custom build, you test the wrong thing.
- We didn’t overfit to a single metric. Three assertions give us high signal with low noise; more can come later.

What I’d like next
- Longer soaks with watch streams in nightly, to catch slow back‑pressure mistakes.
- Label clarity for delete events in `watch_events_total`.
- A resume counter for SSE (we only count gRPC resumes today) — useful to spot flappy connections.
- When clustering lands, switch from pause to true partitions and add Jepsen‑style split‑brain workloads.

The payoff
This chaos CI runs on every PR and nightly. It doesn’t touch the production image, doesn’t require a bespoke environment, and still forces the server through the roughest edges a single node should handle. If a durability bug sneaks in, I want it to fail here loudly, not in a postmortem.

If you’re debating when to add chaos: before clustering is not “too early.” It’s the right time to make correctness visible and routine.

