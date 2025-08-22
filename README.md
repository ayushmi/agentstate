# 🤖 AgentState v1.0.0
**Firebase for AI Agents** — Persistent state management for AI applications

[![Docker Build](https://img.shields.io/badge/docker-ready-blue)](#docker-deployment)
[![API Status](https://img.shields.io/badge/api-stable-green)](#api-reference)
[![Load Tested](https://img.shields.io/badge/load%20tested-1400%20ops%2Fs-brightgreen)](#performance)

AgentState provides a simple, scalable way to store and manage AI agent state with real-time updates, rich querying, and built-in persistence. Think Firebase for your AI agents.

## ✨ Features

- 🔄 **Real-time state updates** - Subscribe to agent state changes
- 🏷️ **Rich querying** - Query agents by tags and attributes  
- 💾 **Persistent storage** - Crash-safe WAL + snapshots
- ⚡ **High performance** - 1,400+ ops/sec, ~15ms latency
- 🐳 **Production ready** - Docker, Kubernetes, monitoring
- 🔌 **Simple API** - HTTP REST + gRPC, language agnostic

## 🚀 Quick Start

### 1. Start AgentState Server

```bash
# Using Docker (recommended)
docker run -p 8080:8080 -p 9090:9090 agentstate:latest

# With persistent storage  
docker run -p 8080:8080 -p 9090:9090 \
  -e DATA_DIR=/data \
  -v agentstate-data:/data \
  agentstate:latest

# Test it works
curl http://localhost:8080/health
```

Authentication in local dev (using docker-compose defaults): generate a dev token and export it as `AGENTSTATE_API_KEY`.

```bash
export AGENTSTATE_API_KEY=$(python scripts/generate_cap_token.py \
  --kid "${CAP_KEY_ACTIVE_ID:-active}" \
  --secret "${CAP_KEY_ACTIVE:-dev-secret}" \
  --ns my-app \
  --verb put --verb get --verb delete --verb query --verb lease)
```

### 2. Use in Your Application

**Python:**
```python
import requests

# Create agent
response = requests.post("http://localhost:8080/v1/my-app/objects", json={
    "type": "chatbot",
    "body": {"name": "CustomerBot", "status": "active"},
    "tags": {"team": "customer-success"}
})
agent = response.json()

# Query agents  
response = requests.post("http://localhost:8080/v1/my-app/query", json={
    "tags": {"team": "customer-success"}
})
agents = response.json()
print(f"Found {len(agents)} customer success agents")
```

**Node.js:**
```javascript
const axios = require('axios');

// Create agent
const {data: agent} = await axios.post('http://localhost:8080/v1/my-app/objects', {
    type: 'workflow',
    body: {name: 'DataProcessor', status: 'idle'},
    tags: {capability: 'data-processing'}
});

// Update agent state
await axios.post('http://localhost:8080/v1/my-app/objects', {
    type: 'workflow', 
    body: {name: 'DataProcessor', status: 'processing', currentJob: 'user-analytics'},
    tags: {capability: 'data-processing'}
}, {params: {id: agent.id}});
```

## 📊 Performance

Real-world benchmarks from our test suite:

- **🚀 Write throughput**: 1,400+ ops/sec
- **🔍 Read throughput**: 170+ queries/sec  
- **⚡ Average latency**: ~15ms
- **📈 P95 latency**: ~30ms
- **✅ Reliability**: 0% error rate under load

## 🏗️ Core Concepts

### Agents as Objects
Each agent is stored with:
- **`id`**: Unique identifier (ULID)
- **`type`**: Agent category ("chatbot", "workflow", etc.)
- **`body`**: Your agent's state (any JSON)
- **`tags`**: Key-value pairs for querying
- **`commit_ts`**: Last update timestamp

### Namespaces
Organize agents by environment/team:
- `/v1/production/objects` - Production agents
- `/v1/staging/objects` - Staging environment
- `/v1/team-alpha/objects` - Team-specific

### Real-time Queries
```python
# Find all active chatbots
response = requests.post("http://localhost:8080/v1/production/query", json={
    "tags": {"type": "chatbot", "status": "active"}
})

# Monitor agents by team
team_agents = requests.post("http://localhost:8080/v1/production/query", json={
    "tags": {"team": "ml-platform"}
}).json()
```

## 🛠️ API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/{ns}/objects` | Create/update agent |
| `GET` | `/v1/{ns}/objects/{id}` | Get agent by ID |
| `POST` | `/v1/{ns}/query` | Query agents by tags |
| `DELETE` | `/v1/{ns}/objects/{id}` | Delete agent |
| `GET` | `/health` | Health check |
| `GET` | `/metrics` | Prometheus metrics |

## 🐳 Docker Deployment

### Basic Setup
```bash
docker run -d --name agentstate \
  -p 8080:8080 \
  -p 9090:9090 \
  agentstate:latest
```

### Production Setup
```bash
docker run -d --name agentstate \
  -p 8080:8080 \
  -p 9090:9090 \
  -e DATA_DIR=/data \
  -v agentstate-data:/data \
  --restart unless-stopped \
  agentstate:latest
```

### Docker Compose
```yaml
version: '3.8'
services:
  agentstate:
    image: agentstate:latest
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - DATA_DIR=/data
    volumes:
      - agentstate-data:/data
    restart: unless-stopped

volumes:
  agentstate-data:
```

## ☸️ Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentstate
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agentstate
  template:
    metadata:
      labels:
        app: agentstate
    spec:
      containers:
      - name: agentstate
        image: agentstate:latest
        ports:
        - containerPort: 8080
        - containerPort: 9090
        env:
        - name: DATA_DIR
          value: /data
        volumeMounts:
        - name: data
          mountPath: /data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: agentstate-data
---
apiVersion: v1
kind: Service
metadata:
  name: agentstate
spec:
  selector:
    app: agentstate
  ports:
  - name: http
    port: 8080
    targetPort: 8080
  - name: grpc  
    port: 9090
    targetPort: 9090
```

## 🔧 Building from Source

### Prerequisites
- Rust 1.81+
- Protocol Buffers compiler

### Build and Run
```bash
# Clone repository
git clone https://github.com/your-org/agentstate.git
cd agentstate

# Build server
cargo build --release -p agentstate-server

# Run server
./target/release/agentstate-server

# Or build Docker image
docker build -f docker/Dockerfile -t agentstate:latest .
```

## 📚 Documentation

- **[📖 Quickstart Guide](QUICKSTART.md)** - Detailed getting started
- **[🏗️ Architecture](docs/architecture.md)** - System design
- **[🚀 Deployment](docs/DEPLOY.md)** - Production setup
- **[📊 Monitoring](deploy/grafana/)** - Grafana dashboards
- **[🔧 Configuration](docs/configuration.md)** - Settings reference

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Integration tests  
python integration_tests.py

# Load testing
python load_test.py

# SDK examples
python examples/quickstart/python_example.py
node examples/quickstart/nodejs_example.js

# Basic test suite
bash test_suite.sh
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🌟 Why AgentState?

Traditional approaches to AI agent state management involve:
- Complex Redis/Postgres setups
- Custom queuing systems  
- Manual state synchronization
- No built-in querying capabilities

AgentState provides:
- ✅ **Simple API** - Just HTTP requests, no complex SDKs
- ✅ **Built-in persistence** - Automatic WAL + snapshots
- ✅ **Rich querying** - Find agents by any tag combination
- ✅ **Real-time updates** - Subscribe to state changes
- ✅ **Production ready** - Monitoring, clustering, reliability
- ✅ **Language agnostic** - Works with any HTTP client

**Perfect for:**
- Multi-agent AI systems
- Agent monitoring dashboards  
- Workflow orchestration
- Real-time agent coordination
- Production AI deployments

---

**Ready to power your AI agents with persistent, queryable state!** 🚀

For questions and support, see our [Issues](https://github.com/your-org/agentstate/issues) page.
