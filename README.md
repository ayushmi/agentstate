# AgentState Cloud (MVP Scaffold)
 
Agentic state cloud scaffold aiming at a Snowflake/Redis-level substrate for agent memory. This repo provides:
 
- Rust workspace: `core`, `storage`, `server` (HTTP JSON API)
- SDKs: Python and TypeScript (MVP)
- gRPC proto definitions (for future hot-paths)
- Dockerfile and Compose for local run
- Docs: quickstart, architecture, consistency semantics, Jepsen test plan
 
This is an MVP scaffold to enable rapid iteration. It favors a local, single-node in-memory engine with a clear separation layer to swap in durable engines.
 
# AgentState — Durable state for agents (Firebase-easy, prod-safe)

Give your agent a persistent, typed object heap with watch streams and vector fields.
No Redis/Postgres glue. No custom queues. Just `state.put()`, `state.query()`, `state.watch()`.

- Crash-safe WAL + snapshots
- Watch with resume tokens (gRPC/SSE)
- Idempotency + leases (fencing)
- Capability tokens (size/QPS/region), TLS/mTLS
- Helm, Grafana, `make verify` harness

## Quickstart (local)
 
- Build server: `cargo run -p agentstate-server` (requires Rust + network to fetch deps)
- Run via Docker: `docker compose up --build`
- Python SDK example:
 
```python
from agentstate import State
s = State("http://localhost:8080/v1/acme")
obj = s.put("note", {"text": "hello"}, tags={"topic":"demo"})
print(s.get(obj["id"]))
```
 
- TypeScript SDK example:
 
```ts
import { State } from "@agentstate/sdk";
const s = new State("http://localhost:8080/v1/acme");
const obj = await s.put("note", {text:"hello"}, {topic:"demo"});
console.log(await s.get(obj.id));
```
 
## Status
 
- HTTP API: put/get/query/watch (MVP)
- Storage: in-memory MVCC with TTL checks (best-effort)
- Watch: gRPC streaming with resume tokens (+ SSE fallback)
- gRPC/proto: defined, not wired yet
 
See `docs/quickstart.md`, `docs/indexes.md`, and `docs/watch.md` for details.
