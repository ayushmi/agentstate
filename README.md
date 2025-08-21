# AgentState — Durable state for agents (Firebase-easy, prod-safe)

Give your agent a persistent, typed object heap with watch streams and vector fields.
No Redis/Postgres glue. No custom queues. Just `state.put()`, `state.query()`, `state.watch()`.

- Crash-safe WAL + snapshots
- Watch with resume tokens (gRPC/SSE)
- Idempotency + leases (fencing)
- Capability tokens (size/QPS/region), TLS/mTLS
- Helm, Grafana, `make verify` harness

## 🚀 10-Minute Quickstart (Docker)

```bash
# Generate admin key
openssl rand -base64 32

# Run AgentState
docker run -d --name agentstate \
  -p 8080:8080 -p 9090:9090 \
  -e CAP_KEY_ACTIVE=<your-generated-key> \
  -v agentstate-data:/data \
  ghcr.io/REPLACE_ORG/agentstate-server:v0.1.0-rc.1

# Test it works
curl http://localhost:8080/health
```

**Python Example:**
```python
import agentstate
client = agentstate.Client("http://localhost:8080")
obj = client.put("agent://demo", {"status": "ready"})
for event in client.watch("agent://demo"):
    print(f"Got {event.type}: {event.body}")
```

**TypeScript Example:**
```typescript
import { Client } from '@agentstate/client';
const client = new Client('http://localhost:8080');
const obj = await client.put('agent://demo', {status: 'ready'});
for await (const event of client.watch('agent://demo')) {
  console.log(`Got ${event.type}:`, event.body);
}
```

## 📚 Documentation

- **[Deployment Guide](docs/DEPLOY.md)** - Docker & Kubernetes setup  
- **[Compatibility Matrix](docs/compatibility.md)** - Supported environments
- **[Architecture](docs/architecture.md)** - System design and consistency
- **[Grafana Dashboard](deploy/grafana/agentstate-dashboard.json)** - Monitoring setup

## 🎯 Helm Installation

```bash
helm upgrade --install agentstate oci://ghcr.io/REPLACE_ORG/charts/agentstate \
  --set env.CAP_KEY_ACTIVE=<your-key> \
  --set persistence.enabled=true
```

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
