"""
AgentState storage adapter for the OpenAI Agents SDK.

Stores agent runs as AgentState objects so they can be retrieved, listed,
and audited across process restarts.

Install:
    pip install openai-agentstate

Usage:
    from agentstate import AgentStateClient
    from openai_agentstate import AgentStateStore

    client = AgentStateClient("http://localhost:8080", namespace="openai-runs")
    store = AgentStateStore(client)

    # Save a run
    store.save_run("run_abc123", {"status": "completed", "output": "Paris"})

    # Retrieve it later
    run = store.get_run("run_abc123")

    # List all completed runs for an agent
    runs = store.list_runs(agent_name="research-agent", status="completed")

Storage layout per run:
    type = "openai_run"
    id   = run_id
    tags = {"status": <status>, "agent_name": <agent_name>}  (populated from run_data)
    body = run_data (the full run dict as-is)
"""

from typing import Any, Dict, List, Optional

from agentstate import AgentStateClient


class AgentStateStore:
    """
    Run storage backend for the OpenAI Agents SDK backed by AgentState.

    This class is intentionally duck-typed (not inheriting from a base class)
    so it remains compatible across OpenAI SDK versions as the interface evolves.

    Attach it to a Runner:
        runner = Runner(agent=my_agent, run_storage=AgentStateStore(client))
    """

    def __init__(self, client: AgentStateClient):
        """
        Args:
            client: AgentStateClient connected to your AgentState server.
                    The client's namespace is used for all run storage.
        """
        self._client = client

    def save_run(self, run_id: str, run_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Persist a run. Calling this twice with the same run_id updates in place.

        Args:
            run_id:   Unique run identifier (e.g., from OpenAI's run object).
            run_data: The full run dict to persist (any JSON-serializable structure).

        Returns:
            The AgentState object representing the stored run.
        """
        tags: Dict[str, str] = {}
        status = run_data.get("status")
        if status is not None:
            tags["status"] = str(status)

        agent_name = (
            run_data.get("agent_name")
            or (run_data.get("agent") or {}).get("name")
        )
        if agent_name:
            tags["agent_name"] = str(agent_name)

        return self._client.create_agent(
            agent_type="openai_run",
            body=run_data,
            tags=tags,
            agent_id=run_id,
            cause={"actor": "openai-agents-sdk", "note": f"run {run_id} status={status}"},
        )

    def get_run(self, run_id: str) -> Optional[Dict[str, Any]]:
        """
        Retrieve the body of a single run by ID.

        Returns:
            The run_data dict that was passed to save_run(), or None if not found.
        """
        try:
            obj = self._client.get_agent(run_id)
            return obj.get("body")
        except Exception:
            return None

    def list_runs(
        self,
        agent_name: Optional[str] = None,
        status: Optional[str] = None,
        limit: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """
        List stored runs, optionally filtered by agent name and/or status.

        Args:
            agent_name: Only return runs for this agent.
            status:     Only return runs with this status (e.g., "completed", "failed").
            limit:      Maximum number of runs to return.

        Returns:
            List of run_data dicts (the bodies, not the AgentState wrappers).
        """
        tags: Dict[str, str] = {}
        if agent_name:
            tags["agent_name"] = agent_name
        if status:
            tags["status"] = status

        results = self._client.query_agents(tags=tags if tags else None)
        if limit:
            results = results[:limit]
        return [r.get("body", {}) for r in results]

    def delete_run(self, run_id: str) -> None:
        """
        Delete a run by ID.

        Args:
            run_id: Unique run identifier to delete.
        """
        self._client.delete_agent(run_id)
