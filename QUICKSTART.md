# 🚀 AgentState Quickstart Guide

**"Firebase for AI Agents"** - Persistent state management for AI applications.

AgentState provides a simple, scalable way to store and manage AI agent state with real-time updates, rich querying, and built-in persistence.

## 🎯 What is AgentState?

AgentState is designed for AI applications that need to:
- **Store agent state persistently** across restarts and deployments
- **Query agents by tags and attributes** for monitoring and coordination  
- **Subscribe to real-time state changes** for reactive applications
- **Scale horizontally** with production-ready reliability

Think of it as a specialized database for AI agents, similar to how Firebase simplified app development.

## ⚡ Quick Start

### 1. Start AgentState Server

Using Docker (recommended):
```bash
docker run -p 8080:8080 ayushmi/agentstate:latest
```

Or with persistent storage:
```bash
docker run -p 8080:8080 -p 9090:9090 \
  -e DATA_DIR=/data \
  -v agentstate-data:/data \
  ayushmi/agentstate:latest
```

### 2. Install SDK

**Python:**
```bash
pip install requests  # AgentState uses standard HTTP APIs
```

**Node.js:**
```bash
npm install axios     # AgentState uses standard HTTP APIs
```

### 2.5 Local Auth (Dev Token)

When running via our `docker-compose.yml`, capability enforcement is enabled by default with a local dev secret. Generate a token and export it as `AGENTSTATE_API_KEY`:

```bash
# Optional: copy defaults
cp .env.example .env  # contains CAP_KEY_ACTIVE=dev-secret

# Generate token for the example namespaces
python scripts/generate_cap_token.py \
  --kid "${CAP_KEY_ACTIVE_ID:-active}" \
  --secret "${CAP_KEY_ACTIVE:-dev-secret}" \
  --ns my-app --ns integration-test \
  --verb put --verb get --verb delete --verb query --verb lease

# Export it
export AGENTSTATE_API_KEY=$(python scripts/generate_cap_token.py \
  --kid "${CAP_KEY_ACTIVE_ID:-active}" \
  --secret "${CAP_KEY_ACTIVE:-dev-secret}" \
  --ns my-app --ns integration-test \
  --verb put --verb get --verb delete --verb query --verb lease)
```

If you prefer to disable auth in local dev, unset capability keys in compose (see comments in `docker-compose.yml`).

### 3. Your First Agent

**Python Example:**
```python
import requests

# Simple client
class AgentState:
    def __init__(self, base_url="http://localhost:8080", namespace="my-app"):
        self.base_url = base_url
        self.namespace = namespace
    
    def create_agent(self, type, body, tags=None):
        response = requests.post(f"{self.base_url}/v1/{self.namespace}/objects", json={
            "type": type, "body": body, "tags": tags or {}
        })
        return response.json()
    
    def get_agent(self, id):
        response = requests.get(f"{self.base_url}/v1/{self.namespace}/objects/{id}")
        return response.json()
    
    def query_agents(self, tags=None):
        response = requests.post(f"{self.base_url}/v1/{self.namespace}/query", json={
            "tags": tags or {}
        })
        return response.json()

# Use it
client = AgentState()

# Create an AI agent
bot = client.create_agent(
    type="chatbot",
    body={
        "name": "CustomerBot",
        "model": "llm-model-v1",
        "status": "active",
        "conversations_today": 0
    },
    tags={
        "team": "customer-success",
        "environment": "production"
    }
)

print(f"Created agent: {bot['id']}")

# Query agents by team
team_agents = client.query_agents({"team": "customer-success"})
print(f"Found {len(team_agents)} agents on customer success team")

# Update agent state
updated = client.create_agent(
    type="chatbot",
    body={
        "name": "CustomerBot", 
        "model": "llm-model-v1",
        "status": "busy",
        "conversations_today": 5,
        "current_user": "john@example.com"
    },
    tags={"team": "customer-success", "environment": "production"}
)
```

**Node.js Example:**
```javascript
const axios = require('axios');

class AgentState {
    constructor(baseUrl = 'http://localhost:8080', namespace = 'my-app') {
        this.baseUrl = baseUrl;
        this.namespace = namespace;
    }
    
    async createAgent(type, body, tags = {}) {
        const response = await axios.post(`${this.baseUrl}/v1/${this.namespace}/objects`, {
            type, body, tags
        });
        return response.data;
    }
    
    async getAgent(id) {
        const response = await axios.get(`${this.baseUrl}/v1/${this.namespace}/objects/${id}`);
        return response.data;
    }
    
    async queryAgents(tags = {}) {
        const response = await axios.post(`${this.baseUrl}/v1/${this.namespace}/query`, { tags });
        return response.data;
    }
}

// Use it
async function main() {
    const client = new AgentState();
    
    // Create workflow automation agent
    const workflow = await client.createAgent('workflow', {
        name: 'DataProcessor',
        status: 'idle',
        queueSize: 0,
        processedToday: 0
    }, {
        capability: 'data-processing',
        environment: 'production'
    });
    
    console.log(`Created workflow agent: ${workflow.id}`);
    
    // Update with job progress
    const updated = await client.createAgent('workflow', {
        name: 'DataProcessor',
        status: 'processing', 
        queueSize: 5,
        processedToday: 42,
        currentJob: 'user-analytics-batch'
    }, {
        capability: 'data-processing',
        environment: 'production'
    });
    
    console.log('Agent updated with job status');
}

main().catch(console.error);
```

## 🏗️ Core Concepts

### Objects (Agents)
Each agent is stored as an object with:
- **`id`**: Unique identifier (auto-generated ULID)
- **`type`**: Agent category (e.g., "chatbot", "workflow", "classifier")  
- **`body`**: Agent state data (JSON object)
- **`tags`**: Key-value pairs for querying and organization
- **`commit_seq`**: Version number for consistency
- **`commit_ts`**: Timestamp of last update

### Namespaces
Organize agents by application, environment, or team:
- `/v1/production/objects` - Production agents
- `/v1/staging/objects` - Staging agents  
- `/v1/team-alpha/objects` - Team-specific agents

### Tagging Strategy
Use tags for querying and organization:
```json
{
  "environment": "production",
  "team": "ml-platform", 
  "capability": "text-classification",
  "version": "2.1.0",
  "region": "us-east-1"
}
```

## 🔧 API Reference

### Create/Update Agent
```http
POST /v1/{namespace}/objects
Content-Type: application/json

{
  "type": "agent-type",
  "body": { "state": "data" },
  "tags": { "key": "value" },
  "id": "optional-specific-id"
}
```

### Get Agent by ID
```http
GET /v1/{namespace}/objects/{id}
```

### Query Agents by Tags
```http
POST /v1/{namespace}/query
Content-Type: application/json

{
  "tags": { "team": "data-science" }
}
```

### Delete Agent
```http
DELETE /v1/{namespace}/objects/{id}
```

### Health Check
```http
GET /health
```

### Metrics (Prometheus)
```http
GET /metrics
```

## 📊 Production Usage Patterns

### 1. Multi-Agent Systems
```python
# Coordinator agent
coordinator = client.create_agent("coordinator", {
    "active_workers": [],
    "tasks_queued": 50,
    "status": "orchestrating"
}, {"role": "coordinator"})

# Worker agents
for i in range(5):
    worker = client.create_agent("worker", {
        "status": "idle",
        "capacity": 10,
        "processed_today": 0
    }, {"role": "worker", "coordinator": coordinator["id"]})
```

### 2. Agent Monitoring Dashboard
```python
# Get all production agents
agents = client.query_agents({"environment": "production"})

# Group by status
active = [a for a in agents if a["body"]["status"] == "active"]
idle = [a for a in agents if a["body"]["status"] == "idle"] 
error = [a for a in agents if a["body"]["status"] == "error"]

print(f"Active: {len(active)}, Idle: {len(idle)}, Error: {len(error)}")
```

### 3. Agent Health Checks
```python
# Find agents that haven't updated recently
import time
current_time = time.time()

stale_agents = []
for agent in client.query_agents({"environment": "production"}):
    last_update = agent.get("commit_ts", 0)
    if current_time - last_update > 300:  # 5 minutes
        stale_agents.append(agent)

print(f"Found {len(stale_agents)} stale agents")
```

## 🚀 Deployment Options

### Docker Compose
```yaml
version: '3.8'
services:
  agentstate:
    image: agentstate:latest
    ports:
      - "8080:8080"  # HTTP API
      - "9090:9090"  # gRPC API  
    environment:
      - DATA_DIR=/data
    volumes:
      - agentstate-data:/data
    restart: unless-stopped

volumes:
  agentstate-data:
```

### Kubernetes
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
```

## 📈 Performance & Scaling

### Benchmarks (Single Instance)
- **Write throughput**: ~1,400 ops/sec
- **Read throughput**: ~170 queries/sec  
- **Mixed workload**: ~120 ops/sec
- **Average latency**: ~15ms
- **P95 latency**: ~30ms

### Scaling Strategies
1. **Vertical scaling**: More CPU/memory for single instance
2. **Read replicas**: Multiple read-only instances (coming soon)
3. **Namespace sharding**: Separate instances per team/environment
4. **Load balancing**: HTTP proxy for multiple instances

## 🔧 Configuration

### Environment Variables
- `DATA_DIR`: Directory for persistent storage (default: in-memory)
- `OTLP_ENDPOINT`: OpenTelemetry endpoint for tracing
- `LOG_LEVEL`: Logging level (default: info)

### Ports
- `8080`: HTTP API (REST)
- `9090`: gRPC API (streaming/high-performance)

## 🛠️ Troubleshooting

### Common Issues

**Connection refused:**
```bash
# Check if server is running
curl http://localhost:8080/health
```

**Slow queries:**
```bash
# Check metrics for bottlenecks  
curl http://localhost:8080/metrics | grep query
```

**Storage issues:**
```bash
# Check disk space for persistent storage
docker exec agentstate-container df -h /data
```

## 🎯 Next Steps

1. **Try the examples**: Run `python examples/quickstart/python_example.py`
2. **Run tests**: Execute `python integration_tests.py`
3. **Load testing**: Try `python load_test.py`
4. **Production deployment**: Use Docker Compose or Kubernetes configs
5. **Monitoring**: Set up Prometheus scraping of `/metrics`

## 🤝 Community & Support

- **Issues**: [GitHub Issues](https://github.com/your-org/agentstate/issues)
- **Documentation**: [Full docs](https://docs.agentstate.dev)
- **Examples**: See `examples/` directory
- **Tests**: Run the test suite for validation

---

**Ready to build the next generation of AI applications with persistent agent state!** 🚀
