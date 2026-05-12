# langgraph-agentstate

LangGraph checkpoint saver backed by [AgentState](https://github.com/ayushmi/agentstate).

Persist and resume any LangGraph graph across process restarts with one line of code.

## Install

```bash
pip install langgraph-agentstate
```

## Quick Start

```python
from agentstate import AgentStateClient
from langgraph_agentstate import AgentStateCheckpointSaver
from langgraph.graph import StateGraph, END
from typing import TypedDict

class State(TypedDict):
    messages: list
    step: int

# 1. Connect to AgentState
client = AgentStateClient("http://localhost:8080", namespace="default")
saver = AgentStateCheckpointSaver(client, namespace="langgraph")

# 2. Build your graph with the checkpoint saver
def my_node(state: State) -> State:
    return {"messages": state["messages"], "step": state["step"] + 1}

graph = (
    StateGraph(State)
    .add_node("node", my_node)
    .set_entry_point("node")
    .add_edge("node", END)
    .compile(checkpointer=saver)
)

# 3. Run — checkpoints are persisted automatically
config = {"configurable": {"thread_id": "my-thread"}}
result = graph.invoke({"messages": [], "step": 0}, config=config)
print("Step:", result["step"])  # 1

# Restart your process — graph resumes from the last checkpoint
result = graph.invoke({"messages": [], "step": result["step"]}, config=config)
print("Step:", result["step"])  # 2
```

## How It Works

Each checkpoint is stored as an AgentState object:

| Field | Value |
|-------|-------|
| `type` | `"langgraph_checkpoint"` |
| `id` | `"{thread_id}:{checkpoint_id}"` |
| `tags` | `{"thread_id": "...", "checkpoint_ns": "..."}` |
| `body` | `{"checkpoint": {...}, "metadata": {...}}` |

This means you can inspect, query, and time-travel through checkpoints using the standard AgentState API.

## Requirements

- AgentState server running (see [quickstart](https://github.com/ayushmi/agentstate#quick-start))
- `langgraph >= 0.1.0`
- `langchain-core >= 0.1.0`
