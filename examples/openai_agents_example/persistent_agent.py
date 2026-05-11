"""
Demonstrates AgentStateStore for persisting OpenAI agent runs.

This example shows the storage layer working independently.
In production, attach the store to your Runner via run_storage=store.

Requirements:
    pip install agentstate openai-agentstate

Start AgentState first:
    docker run -p 8080:8080 ayushmi/agentstate:latest
"""

import os
import time
import uuid

from agentstate import AgentStateClient
from openai_agentstate import AgentStateStore


def main():
    url = os.environ.get("AGENTSTATE_URL", "http://localhost:8080")
    client = AgentStateClient(url, namespace="openai-runs")
    store = AgentStateStore(client)

    # Simulate saving two runs from different agents
    runs = [
        {
            "run_id": f"run_{uuid.uuid4().hex[:8]}",
            "agent_name": "research-agent",
            "status": "completed",
            "input": "What is the capital of France?",
            "output": "Paris",
            "model": "gpt-4o",
            "usage": {"input_tokens": 42, "output_tokens": 5},
            "completed_at": time.time(),
        },
        {
            "run_id": f"run_{uuid.uuid4().hex[:8]}",
            "agent_name": "research-agent",
            "status": "failed",
            "input": "What is 2 + 2?",
            "error": "Rate limit exceeded",
            "model": "gpt-4o",
            "completed_at": time.time(),
        },
    ]

    print("Saving runs...")
    for run in runs:
        rid = run.pop("run_id")
        obj = store.save_run(rid, run)
        print(f"  Saved run {rid} → AgentState ID: {obj['id']}")
        run["run_id"] = rid  # restore for later

    print()

    # Retrieve a specific run
    rid = runs[0]["run_id"]
    retrieved = store.get_run(rid)
    print(f"Retrieved run {rid}:")
    print(f"  agent: {retrieved['agent_name']}, status: {retrieved['status']}")
    print(f"  output: {retrieved.get('output', 'N/A')}")
    print()

    # List only completed runs
    completed = store.list_runs(agent_name="research-agent", status="completed")
    print(f"Completed runs for research-agent: {len(completed)}")

    # List all runs
    all_runs = store.list_runs(agent_name="research-agent")
    print(f"Total runs for research-agent: {len(all_runs)}")

    # Delete one
    store.delete_run(runs[1]["run_id"])
    print(f"\nDeleted failed run {runs[1]['run_id']}")
    all_runs_after = store.list_runs(agent_name="research-agent")
    print(f"Total runs after delete: {len(all_runs_after)}")


if __name__ == "__main__":
    main()
