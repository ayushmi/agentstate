# 🤖 AgentState v1.0.0
**Firebase for AI Agents** — Persistent state management for AI applications

[![Docker Build](https://img.shields.io/badge/docker-ready-blue)](#docker-deployment)
[![API Status](https://img.shields.io/badge/api-stable-green)](#api-reference)  
[![Load Tested](https://img.shields.io/badge/load%20tested-1400%20ops%2Fs-brightgreen)](#performance)
[![Python SDK](https://img.shields.io/badge/python-pypi-blue)](https://pypi.org/project/agentstate/)
[![Node.js SDK](https://img.shields.io/badge/nodejs-npm-green)](https://www.npmjs.com/package/agentstate)

AgentState provides a simple, scalable way to store and manage AI agent state with real-time updates, rich querying, and built-in persistence. Think Firebase for your AI agents.

**🚀 Key Features:**
- **Zero-config setup** — Docker one-liner gets you started
- **Language agnostic** — HTTP/gRPC APIs + Python/Node.js SDKs  
- **High performance** — 1,400+ ops/sec with crash-safe persistence
- **Real-time queries** — Find agents by tags, get live updates
- **Production ready** — Load tested, monitored, Kubernetes friendly

## ✨ Features

- 🔄 **Real-time state updates** - Subscribe to agent state changes
- 🏷️ **Rich querying** - Query agents by tags and attributes  
- 💾 **Persistent storage** - Crash-safe WAL + snapshots
- ⚡ **High performance** - 1,400+ ops/sec, ~15ms latency
- 🐳 **Production ready** - Docker, Kubernetes, monitoring
- 🔌 **Simple API** - HTTP REST + gRPC, language agnostic

## 🚀 Quick Start

### 1. Start AgentState Server

**Option A: Using Docker (Recommended)**
```bash
# Quick start - no auth required
docker run -p 8080:8080 ayushmi/agentstate:latest

# With persistent storage
docker run -p 8080:8080 -p 9090:9090 \
  -e DATA_DIR=/data \
  -v agentstate-data:/data \
  ayushmi/agentstate:latest

# Test it works
curl http://localhost:8080/health
```

**Option B: Using Docker Compose (Full Setup)**
```bash
git clone https://github.com/ayushmi/agentstate.git
cd agentstate
docker-compose up -d

# Generate auth token for testing (optional)
export AGENTSTATE_API_KEY=$(python scripts/generate_cap_token.py \
  --kid active --secret dev-secret \
  --ns my-app --verb put --verb get --verb delete --verb query --verb lease)
```

### 2. Use in Your Application

**Python SDK:**
```bash
pip install agentstate
```
```python
from agentstate import AgentStateClient

client = AgentStateClient(base_url='http://localhost:8080', namespace='my-app')

# Create agent
agent = client.create_agent(
    agent_type='chatbot',
    body={'name': 'CustomerBot', 'status': 'active'},
    tags={'team': 'customer-success'}
)
print(f"Created agent: {agent['id']}")

# Query agents  
agents = client.query_agents(tags={'team': 'customer-success'})
print(f"Found {len(agents)} customer success agents")

# Get specific agent
agent = client.get_agent(agent_id)
print(f"Agent status: {agent['body']['status']}")
```

**Node.js SDK:**
```bash
npm install agentstate
```
```javascript
import { AgentStateClient } from 'agentstate';

const client = new AgentStateClient({
    baseUrl: 'http://localhost:8080',
    namespace: 'my-app'
});

// Create agent
const agent = await client.createAgent({
    type: 'workflow',
    body: {name: 'DataProcessor', status: 'idle'},
    tags: {capability: 'data-processing'}
});

// Update agent state
const updatedAgent = await client.updateAgent(agent.id, {
    body: {name: 'DataProcessor', status: 'processing', currentJob: 'analytics'}
});

console.log(`Agent ${agent.id} status: ${updatedAgent.body.status}`);
```

**Raw HTTP API:**
```bash
# Create agent
curl -X POST http://localhost:8080/v1/my-app/objects \
  -H "Content-Type: application/json" \
  -d '{"type": "chatbot", "body": {"name": "Bot1"}, "tags": {"env": "prod"}}'

# Query agents
curl -X POST http://localhost:8080/v1/my-app/query \
  -H "Content-Type: application/json" \
  -d '{"tags": {"env": "prod"}}'
```

## 🤖 AI Framework Integration

AgentState integrates seamlessly with popular AI frameworks:

**LangChain Integration:**
```python
from agentstate import AgentStateClient
from langchain.memory import BaseChatMessageHistory
from langchain.agents import AgentExecutor

# Use AgentState as LangChain memory backend
class AgentStateMemory(BaseChatMessageHistory):
    def __init__(self, agent_id: str, client: AgentStateClient):
        self.agent_id = agent_id
        self.client = client

# Full LangChain + AgentState demo available in examples/
```

**CrewAI Integration:**
```python
from agentstate import AgentStateClient
import crewai

client = AgentStateClient(base_url='http://localhost:8080', namespace='crew')

# Store crew member states, task progress, and coordination
agent = client.create_agent(
    agent_type='crew_member',
    body={'role': 'researcher', 'current_task': 'market_analysis'},
    tags={'crew_id': 'marketing_team', 'status': 'active'}
)
```

**Custom Agent Frameworks:**
```python
# AgentState works with any agent framework
class MyAgent:
    def __init__(self, agent_id):
        self.state = AgentStateClient(namespace='my_agents')
        self.id = agent_id
        
    def save_state(self, data):
        return self.state.create_agent(
            agent_type='custom',
            body=data,
            agent_id=self.id
        )
        
    def load_state(self):
        return self.state.get_agent(self.id)
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
git clone https://github.com/ayushmi/agentstate.git
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

## 💡 Use Cases

**Multi-Agent AI Systems:**
```python
# Coordinate multiple specialized agents
marketing_agent = client.create_agent('marketing_specialist', {...})
research_agent = client.create_agent('research_specialist', {...})
writer_agent = client.create_agent('content_writer', {...})

# Query agents by capability when needed
available_agents = client.query_agents(tags={'status': 'idle', 'capability': 'research'})
```

**Workflow Orchestration:**
```python
# Track workflow steps and state
workflow = client.create_agent(
    agent_type='workflow',
    body={
        'current_step': 'data_collection',
        'completed_steps': ['initialization'],
        'next_steps': ['analysis', 'reporting']
    },
    tags={'workflow_id': 'user_onboarding', 'priority': 'high'}
)
```

**Agent Monitoring & Analytics:**
```python
# Real-time agent health monitoring
active_agents = client.query_agents(tags={'status': 'active'})
failed_agents = client.query_agents(tags={'status': 'error'})

# Build dashboards with live agent metrics
for agent in active_agents:
    print(f"Agent {agent['id']}: {agent['body']['current_task']}")
```

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

## 🚀 Try it Now

**1-Minute Setup:**
```bash
# Start server
docker run -p 8080:8080 ayushmi/agentstate:latest

# Install SDK (Python or Node.js)
pip install agentstate
# npm install agentstate

# Create your first agent
python -c "
from agentstate import AgentStateClient
client = AgentStateClient(base_url='http://localhost:8080', namespace='demo')
agent = client.create_agent('chatbot', {'name': 'MyBot', 'status': 'active'})
print(f'Created agent: {agent[\"id\"]}')
"
```

**Explore Examples:**
- 🦜 **LangChain Integration**: [AgentStateTesting/python-tests/langchain-example/](AgentStateTesting/python-tests/langchain-example/)
- 🤖 **CrewAI Integration**: [AgentStateTesting/python-tests/crewai-example/](AgentStateTesting/python-tests/crewai-example/)  
- 📝 **Complete Quickstart**: [QUICKSTART.md](QUICKSTART.md)

For questions and support, see our [Issues](https://github.com/ayushmi/agentstate/issues) page.
