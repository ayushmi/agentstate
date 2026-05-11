"""
LangGraph agent that survives process restarts by checkpointing to AgentState.

Run this script twice — the step counter will continue from where it left off.

Requirements:
    pip install agentstate langgraph-agentstate langgraph langchain-core

Start AgentState first:
    docker run -p 8080:8080 ayushmi/agentstate:latest
"""

import os
from typing import TypedDict

from agentstate import AgentStateClient
from langgraph_agentstate import AgentStateCheckpointSaver

try:
    from langgraph.graph import StateGraph, END
except ImportError:
    print("Install langgraph: pip install langgraph langchain-core")
    raise


class AgentState(TypedDict):
    messages: list
    step: int


def process(state: AgentState) -> AgentState:
    """Simulate one step of work."""
    print(f"  Processing step {state['step']} → {state['step'] + 1}")
    return {"messages": state["messages"], "step": state["step"] + 1}


def build_graph(saver: AgentStateCheckpointSaver):
    g = StateGraph(AgentState)
    g.add_node("process", process)
    g.set_entry_point("process")
    g.add_edge("process", END)
    return g.compile(checkpointer=saver)


def main():
    url = os.environ.get("AGENTSTATE_URL", "http://localhost:8080")
    client = AgentStateClient(url, namespace="default")
    saver = AgentStateCheckpointSaver(client, namespace="langgraph")
    graph = build_graph(saver)

    config = {"configurable": {"thread_id": "demo-thread-001"}}

    # Try to resume from existing checkpoint, or start fresh
    existing = saver.get_tuple(config)
    if existing:
        current_step = existing.checkpoint.get("channel_values", {}).get("step", 0)
        print(f"Resuming from checkpoint — current step: {current_step}")
        init_state: AgentState = {"messages": [], "step": current_step}
    else:
        print("No checkpoint found — starting fresh")
        init_state = {"messages": [], "step": 0}

    result = graph.invoke(init_state, config=config)
    print(f"Done. Step is now: {result['step']}")
    print("Run this script again to continue from this checkpoint.")


if __name__ == "__main__":
    main()
