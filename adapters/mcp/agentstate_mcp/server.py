"""
AgentState MCP Server

Exposes AgentState as persistent memory tools for any MCP-compatible client
(Claude Desktop, Cursor, custom agents, etc.).

Configure via environment variables:
    AGENTSTATE_URL        URL of the AgentState server (default: http://localhost:8080)
    AGENTSTATE_NAMESPACE  Namespace for memory storage (default: mcp-memory)
    AGENTSTATE_API_KEY    API key for authentication (optional)

Run:
    agentstate-mcp
    # or
    python -m agentstate_mcp.server

Claude Desktop config (~/.claude/claude_desktop_config.json):
    {
      "mcpServers": {
        "agentstate-memory": {
          "command": "agentstate-mcp",
          "env": {
            "AGENTSTATE_URL": "http://localhost:8080",
            "AGENTSTATE_NAMESPACE": "claude-memories"
          }
        }
      }
    }
"""

import asyncio
import json
import os
import sys
from typing import Any

from agentstate import AgentStateClient

try:
    import mcp.server.stdio
    import mcp.types as types
    from mcp.server import Server
except ImportError as e:
    raise ImportError(
        "mcp is required. Install it with: pip install mcp"
    ) from e

AGENTSTATE_URL = os.environ.get("AGENTSTATE_URL", "http://localhost:8080")
AGENTSTATE_NAMESPACE = os.environ.get("AGENTSTATE_NAMESPACE", "mcp-memory")
AGENTSTATE_API_KEY = os.environ.get("AGENTSTATE_API_KEY")

_client = AgentStateClient(
    base_url=AGENTSTATE_URL,
    namespace=AGENTSTATE_NAMESPACE,
    api_key=AGENTSTATE_API_KEY,
)

app = Server("agentstate-memory")


@app.list_tools()
async def list_tools() -> list[types.Tool]:
    return [
        types.Tool(
            name="store_memory",
            description=(
                "Store a memory or piece of information persistently in AgentState. "
                "Returns the memory ID for later retrieval."
            ),
            inputSchema={
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The content or information to store",
                    },
                    "tags": {
                        "type": "object",
                        "description": "Optional key-value tags for categorization and retrieval",
                        "additionalProperties": {"type": "string"},
                    },
                    "memory_id": {
                        "type": "string",
                        "description": "Optional stable ID — if provided, updates the memory in place on subsequent calls",
                    },
                },
                "required": ["content"],
            },
        ),
        types.Tool(
            name="retrieve_memory",
            description="Retrieve a specific memory by its ID.",
            inputSchema={
                "type": "object",
                "properties": {
                    "memory_id": {
                        "type": "string",
                        "description": "The ID of the memory to retrieve",
                    },
                },
                "required": ["memory_id"],
            },
        ),
        types.Tool(
            name="search_memories",
            description=(
                "Search stored memories by tags. All provided tags must match. "
                "Returns a list of matching memories."
            ),
            inputSchema={
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "object",
                        "description": "Tag filters — only memories matching ALL tags are returned",
                        "additionalProperties": {"type": "string"},
                    },
                },
                "required": [],
            },
        ),
        types.Tool(
            name="delete_memory",
            description="Permanently delete a memory by ID.",
            inputSchema={
                "type": "object",
                "properties": {
                    "memory_id": {
                        "type": "string",
                        "description": "The ID of the memory to delete",
                    },
                },
                "required": ["memory_id"],
            },
        ),
        types.Tool(
            name="list_memories",
            description="List all stored memories, optionally filtered by tags.",
            inputSchema={
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "object",
                        "description": "Optional tag filters",
                        "additionalProperties": {"type": "string"},
                    },
                },
                "required": [],
            },
        ),
    ]


@app.call_tool()
async def call_tool(name: str, arguments: dict[str, Any]) -> list[types.TextContent]:
    try:
        if name == "store_memory":
            tags = arguments.get("tags") or {}
            obj = _client.create_agent(
                agent_type="memory",
                body={"content": arguments["content"]},
                tags=tags,
                agent_id=arguments.get("memory_id"),
                cause={"actor": "mcp-client", "note": "stored via MCP tool"},
            )
            return [types.TextContent(
                type="text",
                text=f"Memory stored successfully.\nID: {obj['id']}\nUse this ID to retrieve or update it later.",
            )]

        elif name == "retrieve_memory":
            obj = _client.get_agent(arguments["memory_id"])
            content = obj.get("body", {}).get("content", "")
            return [types.TextContent(
                type="text",
                text=f"Memory (ID: {obj['id']}):\n{content}",
            )]

        elif name == "search_memories":
            tags = arguments.get("tags") or {}
            results = _client.query_agents(tags=tags if tags else None)
            if not results:
                return [types.TextContent(type="text", text="No memories found matching the given tags.")]
            lines = []
            for obj in results:
                content = obj.get("body", {}).get("content", "")
                lines.append(f"[{obj['id']}] {content}")
            return [types.TextContent(type="text", text="\n".join(lines))]

        elif name == "delete_memory":
            _client.delete_agent(arguments["memory_id"])
            return [types.TextContent(
                type="text",
                text=f"Memory {arguments['memory_id']} deleted.",
            )]

        elif name == "list_memories":
            tags = arguments.get("tags") or {}
            results = _client.query_agents(tags=tags if tags else None)
            if not results:
                return [types.TextContent(type="text", text="No memories stored yet.")]
            lines = []
            for obj in results:
                content = obj.get("body", {}).get("content", "")
                tag_str = ", ".join(f"{k}={v}" for k, v in obj.get("tags", {}).items())
                lines.append(f"[{obj['id']}] {content[:80]}{'...' if len(content) > 80 else ''} ({tag_str})")
            return [types.TextContent(type="text", text="\n".join(lines))]

        else:
            return [types.TextContent(type="text", text=f"Unknown tool: {name}")]

    except Exception as e:
        return [types.TextContent(type="text", text=f"Error: {e}")]


async def _run():
    async with mcp.server.stdio.stdio_server() as (read_stream, write_stream):
        await app.run(read_stream, write_stream, app.create_initialization_options())


def main():
    asyncio.run(_run())


if __name__ == "__main__":
    main()
