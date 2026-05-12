# How to Add a New Framework Adapter

Adapters are thin packages that wire a framework's storage interface to AgentState.
Each adapter lives in `adapters/` and is published as a standalone package so users can
install only what they need.

**Existing adapters:**
- `adapters/python/langgraph_agentstate/` — LangGraph checkpoint saver
- `adapters/python/openai_agentstate/` — OpenAI Agents SDK run storage
- `adapters/mcp/` — MCP memory server for Claude Desktop and other MCP clients

---

## Directory Convention

```
adapters/
  <language>/
    <framework>_agentstate/
      <framework>_agentstate/
        __init__.py        # re-exports the main class
        adapter.py         # core implementation
      pyproject.toml       # or package.json / go.mod
      README.md
      tests/
        test_adapter.py

examples/
  <framework>_example/
    main.py
    requirements.txt
```

---

## Step 1: Understand the Target Interface

Read the framework's docs to find the storage or persistence interface:
- What base class or protocol must be implemented?
- Which methods are required vs optional?
- Are async variants needed?

**Examples of interfaces that map well to AgentState:**
- LangGraph: `BaseCheckpointSaver` (thread-based, versioned state)
- OpenAI Agents: `RunStorage` (run_id-keyed records)
- Haystack: `DocumentStore` (vector + metadata storage)
- AutoGen: `MessageStore` (conversation history)

---

## Step 2: Map Framework Concepts to AgentState Fields

| Framework concept | AgentState field |
|------------------|-----------------|
| Run / thread ID | `id` (stable, upserts in place) |
| State type / category | `type` (e.g., `"langgraph_checkpoint"`) |
| State payload | `body` (any JSON) |
| Filtering dimensions | `tags` (key-value string pairs) |
| Who made this change | `cause.actor` |
| Why it changed | `cause.note` |
| What triggered it | `cause.trigger` (commit hash) |

---

## Step 3: Create the Package

Copy the structure from an existing adapter. Update `pyproject.toml`:

```toml
[project]
name = "<framework>-agentstate"
version = "0.1.0"
description = "<Framework> adapter backed by AgentState"
requires-python = ">=3.9"
dependencies = [
    "agentstate>=1.0.2",
    "<framework-package>>=<min-version>",
]
```

---

## Step 4: Implement the Adapter

Minimal Python pattern:

```python
from typing import Any, Dict, List, Optional
from agentstate import AgentStateClient


class AgentStateAdapter:
    """
    <Framework> storage adapter backed by AgentState.

    Storage layout:
      type = "<framework>_state"
      id   = <framework's stable key>
      tags = {<filtering dimensions>}
      body = <framework's state payload>
    """

    def __init__(self, client: AgentStateClient, namespace: str = "<framework>"):
        # Isolate into a dedicated namespace so framework state doesn't mix
        # with other objects in the user's default namespace.
        api_key = client.session.headers.get("Authorization", "").replace("Bearer ", "") or None
        self._client = AgentStateClient(
            base_url=client.base_url,
            namespace=namespace,
            api_key=api_key if api_key else None,
        )

    def save(self, key: str, data: Dict[str, Any], cause: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
        return self._client.create_agent(
            agent_type="<framework>_state",
            body=data,
            agent_id=key,
            cause=cause,
        )

    def load(self, key: str) -> Optional[Dict[str, Any]]:
        try:
            obj = self._client.get_agent(key)
            return obj["body"]
        except Exception:
            return None

    def list(self, tags: Optional[Dict[str, str]] = None) -> List[Dict[str, Any]]:
        results = self._client.query_agents(tags=tags)
        return [r["body"] for r in results]

    def delete(self, key: str) -> None:
        self._client.delete_agent(key)
```

---

## Step 5: Write Tests

Minimum test checklist (all must pass against a live AgentState server):

- [ ] `save()` → `load()` round-trip: same key, same data
- [ ] `save()` twice with same key updates in place (idempotent upsert)
- [ ] `list()` with tag filter returns only matching items
- [ ] `delete()` → `load()` returns `None`
- [ ] Missing key → `load()` returns `None` without raising
- [ ] Server unreachable → method raises a clear exception (not a hang)

```python
# tests/test_adapter.py
import pytest
from agentstate import AgentStateClient
from <framework>_agentstate import AgentStateAdapter


@pytest.fixture
def adapter():
    client = AgentStateClient("http://localhost:8080", namespace="test-<framework>")
    return AgentStateAdapter(client)


def test_round_trip(adapter):
    adapter.save("key-1", {"value": 42})
    assert adapter.load("key-1") == {"value": 42}


def test_upsert(adapter):
    adapter.save("key-2", {"value": 1})
    adapter.save("key-2", {"value": 2})
    assert adapter.load("key-2") == {"value": 2}


def test_missing_key(adapter):
    assert adapter.load("does-not-exist-xyz") is None


def test_delete(adapter):
    adapter.save("key-3", {"value": 99})
    adapter.delete("key-3")
    assert adapter.load("key-3") is None
```

Run with:
```bash
# Start server first
docker run -p 8080:8080 ayushmi/agentstate:latest

pytest adapters/<language>/<framework>_agentstate/tests/ -v
```

---

## Step 6: Add an Example

Create `examples/<framework>_example/main.py` showing a realistic scenario — ideally
one that demonstrates crash recovery or resumption, which is the core value proposition.

The example should be runnable with just:
```bash
pip install -r examples/<framework>_example/requirements.txt
docker run -p 8080:8080 ayushmi/agentstate:latest
python examples/<framework>_example/main.py
```

---

## Step 7: Submit a PR

- **Target branch:** `main`
- **Title:** `feat(adapters): add <framework> adapter`
- **Required in PR description:**
  - Link to the framework's storage interface documentation
  - Screenshot or log output showing the example working end-to-end
  - Test results output (`pytest -v`)

---

## Reviewer Checklist

- [ ] Package installs cleanly: `pip install -e adapters/<language>/<framework>_agentstate`
- [ ] All 6 unit tests pass with a live server
- [ ] Example runs end-to-end without errors
- [ ] Only `AgentStateClient` public API is used (no internal imports)
- [ ] `pyproject.toml` pins a minimum version for `agentstate` and the framework
- [ ] `README.md` includes install instructions, a quick-start snippet, and a storage layout table
- [ ] New namespace is used (isolated from default user namespace)
- [ ] `cause` field is populated where meaningful (actor, note)
