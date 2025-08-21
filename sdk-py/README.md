# 🤖 AgentState Python SDK

**Firebase for AI Agents** - Python SDK for persistent agent state management.

[![PyPI version](https://img.shields.io/pypi/v/agentstate.svg)](https://pypi.org/project/agentstate/)
[![Python versions](https://img.shields.io/pypi/pyversions/agentstate.svg)](https://pypi.org/project/agentstate/)
[![License](https://img.shields.io/pypi/l/agentstate.svg)](https://github.com/ayushmi/agentstate/blob/main/LICENSE)

## 🚀 Quick Start

### Installation

```bash
pip install agentstate
```

### Basic Usage

```python
from agentstate import AgentStateClient

# Connect to AgentState server
client = AgentStateClient("http://localhost:8080", namespace="my-app")

# Create an agent
agent = client.create_agent(
    agent_type="chatbot",
    body={"name": "CustomerBot", "status": "active", "conversations": 0},
    tags={"team": "support", "environment": "production"}
)

print(f"Created agent: {agent['id']}")

# Update agent state
updated = client.create_agent(
    agent_type="chatbot", 
    body={"name": "CustomerBot", "status": "busy", "conversations": 5},
    tags={"team": "support", "environment": "production"},
    agent_id=agent['id']  # Update existing agent
)

# Query agents
support_agents = client.query_agents({"team": "support"})
print(f"Found {len(support_agents)} support agents")

# Get specific agent
retrieved = client.get_agent(agent['id'])
print(f"Agent status: {retrieved['body']['status']}")
```

## 📚 API Reference

### AgentStateClient

#### `__init__(base_url, namespace)`

Initialize the client.

- `base_url`: AgentState server URL (e.g., "http://localhost:8080")
- `namespace`: Namespace for organizing agents (e.g., "production", "staging")

#### `create_agent(agent_type, body, tags=None, agent_id=None)`

Create or update an agent.

- `agent_type`: Agent category (e.g., "chatbot", "workflow", "classifier")
- `body`: Agent state data (dict)
- `tags`: Key-value pairs for querying (dict, optional)
- `agent_id`: Specific ID for updates (str, optional)

Returns: Agent object with `id`, `type`, `body`, `tags`, `commit_seq`, `commit_ts`

#### `get_agent(agent_id)`

Get agent by ID.

- `agent_id`: Unique agent identifier

Returns: Agent object

#### `query_agents(tags=None)`

Query agents by tags.

- `tags`: Tag filters (e.g., `{"team": "support", "status": "active"}`)

Returns: List of matching agent objects

#### `delete_agent(agent_id)`

Delete an agent.

- `agent_id`: Unique agent identifier

#### `health_check()`

Check server health.

Returns: `True` if healthy, `False` otherwise

## 🎯 Usage Examples

### Multi-Agent System

```python
from agentstate import AgentStateClient

client = AgentStateClient("http://localhost:8080", "multi-agent-system")

# Create coordinator agent
coordinator = client.create_agent("coordinator", {
    "status": "active",
    "workers": [],
    "tasks_queued": 50
}, {"role": "coordinator"})

# Create worker agents
workers = []
for i in range(3):
    worker = client.create_agent("worker", {
        "status": "idle",
        "processed_today": 0,
        "coordinator_id": coordinator["id"]
    }, {"role": "worker", "coordinator": coordinator["id"]})
    workers.append(worker)

print("Multi-agent system initialized!")
```

## 🔗 Links

- **GitHub**: https://github.com/ayushmi/agentstate
- **Documentation**: https://github.com/ayushmi/agentstate#readme
- **Issues**: https://github.com/ayushmi/agentstate/issues

