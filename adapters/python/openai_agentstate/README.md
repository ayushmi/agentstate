# openai-agentstate

Persistent run storage for the [OpenAI Agents SDK](https://github.com/openai/openai-agents-python) backed by [AgentState](https://github.com/ayushmi/agentstate).

Store and retrieve agent runs across restarts. Query by agent name, status, or any custom tag.

## Install

```bash
pip install openai-agentstate
```

## Quick Start

```python
from agentstate import AgentStateClient
from openai_agentstate import AgentStateStore

client = AgentStateClient("http://localhost:8080", namespace="openai-runs")
store = AgentStateStore(client)

# Save a run (call this after each run completes)
store.save_run("run_abc123", {
    "agent_name": "research-agent",
    "status": "completed",
    "input": "What is the capital of France?",
    "output": "Paris",
    "model": "gpt-4o",
})

# Retrieve a specific run
run = store.get_run("run_abc123")
print(run["output"])  # "Paris"

# List all completed runs for an agent
runs = store.list_runs(agent_name="research-agent", status="completed")
print(f"Found {len(runs)} completed runs")

# Attach to OpenAI Agents SDK Runner (when run_storage interface is stable)
# from openai_agents import Runner
# runner = Runner(agent=my_agent, run_storage=store)
```

## Storage Layout

Each run is stored as an AgentState object:

| Field | Value |
|-------|-------|
| `type` | `"openai_run"` |
| `id` | the `run_id` you provide |
| `tags` | `{"status": "...", "agent_name": "..."}` |
| `body` | the full `run_data` dict |

## API

### `save_run(run_id, run_data)` → `dict`
Persist a run. Idempotent — calling twice with the same `run_id` updates in place.

### `get_run(run_id)` → `dict | None`
Retrieve the `run_data` for a run by ID. Returns `None` if not found.

### `list_runs(agent_name?, status?, limit?)` → `list[dict]`
List runs, optionally filtered. Returns the `run_data` bodies.

### `delete_run(run_id)` → `None`
Delete a run by ID.
