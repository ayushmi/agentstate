"""
LangGraph checkpoint saver backed by AgentState.

Install:
    pip install langgraph-agentstate

Usage:
    from agentstate import AgentStateClient
    from langgraph_agentstate import AgentStateCheckpointSaver
    from langgraph.graph import StateGraph

    client = AgentStateClient("http://localhost:8080", namespace="default")
    saver = AgentStateCheckpointSaver(client, namespace="langgraph")

    graph = StateGraph(...).compile(checkpointer=saver)
    graph.invoke({...}, config={"configurable": {"thread_id": "thread-1"}})

Storage layout per checkpoint:
    type  = "langgraph_checkpoint"
    id    = "{thread_id}:{checkpoint_id}"
    tags  = {"thread_id": <thread_id>, "checkpoint_ns": <checkpoint_ns>}
    body  = {"checkpoint": <checkpoint dict>, "metadata": <metadata dict>}
"""

import asyncio
from typing import Any, AsyncIterator, Dict, Iterator, List, Optional

from agentstate import AgentStateClient

try:
    from langgraph.checkpoint.base import (
        BaseCheckpointSaver,
        Checkpoint,
        CheckpointMetadata,
        CheckpointTuple,
        SerializerProtocol,
    )
except ImportError as e:
    raise ImportError(
        "langgraph is required. Install it with: pip install langgraph"
    ) from e

try:
    from langchain_core.runnables import RunnableConfig
except ImportError as e:
    raise ImportError(
        "langchain-core is required. Install it with: pip install langchain-core"
    ) from e


class AgentStateCheckpointSaver(BaseCheckpointSaver):
    """
    LangGraph checkpoint saver that persists checkpoints in AgentState.

    Each checkpoint is stored as an AgentState object:
      - type = "langgraph_checkpoint"
      - id   = "{thread_id}:{checkpoint_id}"  (stable, upserts in place)
      - tags = {"thread_id": ..., "checkpoint_ns": ...}
      - body = {"checkpoint": {...}, "metadata": {...}}

    Supports both sync and async LangGraph graph execution.
    """

    def __init__(self, client: AgentStateClient, namespace: str = "langgraph"):
        """
        Args:
            client: An AgentStateClient instance (base_url and api_key are reused).
            namespace: AgentState namespace to store checkpoints in.
        """
        super().__init__()
        # Reuse connection settings from the provided client but isolate namespace
        api_key = client.session.headers.get("Authorization", "").replace("Bearer ", "") or None
        self._client = AgentStateClient(
            base_url=client.base_url,
            namespace=namespace,
            api_key=api_key if api_key else None,
        )

    # -------------------------------------------------------------------------
    # Sync interface (required by BaseCheckpointSaver)
    # -------------------------------------------------------------------------

    def get_tuple(self, config: RunnableConfig) -> Optional[CheckpointTuple]:
        """Return the latest checkpoint tuple for the given thread, or a specific one by ID."""
        configurable = config.get("configurable", {})
        thread_id = configurable.get("thread_id", "")
        checkpoint_id = configurable.get("checkpoint_id")

        if checkpoint_id:
            obj_id = f"{thread_id}:{checkpoint_id}"
            try:
                obj = self._client.get_agent(obj_id)
            except Exception:
                return None
            return self._to_checkpoint_tuple(obj, config)
        else:
            results = self._client.query_agents(tags={"thread_id": thread_id})
            if not results:
                return None
            latest = max(results, key=lambda o: o.get("commit_seq", 0))
            return self._to_checkpoint_tuple(latest, config)

    def put(
        self,
        config: RunnableConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
        new_versions: Dict[str, Any],
    ) -> RunnableConfig:
        """Persist a checkpoint and return the updated config containing the checkpoint_id."""
        configurable = config.get("configurable", {})
        thread_id = configurable.get("thread_id", "")
        checkpoint_ns = configurable.get("checkpoint_ns", "")
        checkpoint_id = checkpoint["id"]
        obj_id = f"{thread_id}:{checkpoint_id}"

        self._client.create_agent(
            agent_type="langgraph_checkpoint",
            body={"checkpoint": checkpoint, "metadata": metadata},
            tags={"thread_id": thread_id, "checkpoint_ns": checkpoint_ns},
            agent_id=obj_id,
            cause={"actor": "langgraph", "note": f"checkpoint {checkpoint_id}"},
        )
        return {
            **config,
            "configurable": {
                **configurable,
                "checkpoint_id": checkpoint_id,
                "checkpoint_ns": checkpoint_ns,
            },
        }

    def put_writes(
        self,
        config: RunnableConfig,
        writes: List[Any],
        task_id: str,
    ) -> None:
        """Store intermediate writes (pending channel values) — stored as a separate object."""
        configurable = config.get("configurable", {})
        thread_id = configurable.get("thread_id", "")
        checkpoint_id = configurable.get("checkpoint_id", "")
        obj_id = f"{thread_id}:{checkpoint_id}:writes:{task_id}"

        self._client.create_agent(
            agent_type="langgraph_pending_write",
            body={"writes": writes, "task_id": task_id},
            tags={"thread_id": thread_id, "checkpoint_id": checkpoint_id},
            agent_id=obj_id,
        )

    def list(
        self,
        config: Optional[RunnableConfig],
        *,
        filter: Optional[Dict[str, Any]] = None,
        before: Optional[RunnableConfig] = None,
        limit: Optional[int] = None,
    ) -> Iterator[CheckpointTuple]:
        """Yield checkpoints for a thread, newest first."""
        if not config:
            return
        thread_id = config.get("configurable", {}).get("thread_id", "")
        results = self._client.query_agents(tags={"thread_id": thread_id})
        # Only langgraph_checkpoint objects (not pending writes)
        results = [r for r in results if r.get("type") == "langgraph_checkpoint"]
        results.sort(key=lambda o: o.get("commit_seq", 0), reverse=True)
        if limit:
            results = results[:limit]
        for obj in results:
            ct = self._to_checkpoint_tuple(obj, config)
            if ct:
                yield ct

    # -------------------------------------------------------------------------
    # Async interface (delegates to sync via run_in_executor for simplicity)
    # -------------------------------------------------------------------------

    async def aget_tuple(self, config: RunnableConfig) -> Optional[CheckpointTuple]:
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.get_tuple, config)

    async def aput(
        self,
        config: RunnableConfig,
        checkpoint: Checkpoint,
        metadata: CheckpointMetadata,
        new_versions: Dict[str, Any],
    ) -> RunnableConfig:
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, self.put, config, checkpoint, metadata, new_versions)

    async def aput_writes(
        self,
        config: RunnableConfig,
        writes: List[Any],
        task_id: str,
    ) -> None:
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, self.put_writes, config, writes, task_id)

    async def alist(
        self,
        config: Optional[RunnableConfig],
        *,
        filter: Optional[Dict[str, Any]] = None,
        before: Optional[RunnableConfig] = None,
        limit: Optional[int] = None,
    ) -> AsyncIterator[CheckpointTuple]:
        loop = asyncio.get_event_loop()
        items = await loop.run_in_executor(
            None,
            lambda: list(self.list(config, filter=filter, before=before, limit=limit)),
        )
        for item in items:
            yield item

    # -------------------------------------------------------------------------
    # Helpers
    # -------------------------------------------------------------------------

    def _to_checkpoint_tuple(
        self, obj: Dict[str, Any], config: RunnableConfig
    ) -> Optional[CheckpointTuple]:
        body = obj.get("body", {})
        checkpoint = body.get("checkpoint")
        metadata = body.get("metadata", {})
        if not checkpoint:
            return None
        tags = obj.get("tags", {})
        thread_id = tags.get("thread_id", "")
        checkpoint_id = checkpoint.get("id", "")
        return CheckpointTuple(
            config={
                "configurable": {
                    "thread_id": thread_id,
                    "checkpoint_id": checkpoint_id,
                    "checkpoint_ns": tags.get("checkpoint_ns", ""),
                }
            },
            checkpoint=checkpoint,
            metadata=metadata,
            parent_config=None,
        )
