# agentstate-mcp

MCP server that exposes [AgentState](https://github.com/ayushmi/agentstate) as persistent memory tools for Claude Desktop, Cursor, and any other MCP-compatible client.

## Install

```bash
pip install agentstate-mcp
```

## Quick Start

### 1. Start AgentState

```bash
docker run -p 8080:8080 ayushmi/agentstate:latest
```

### 2. Configure Claude Desktop

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agentstate-memory": {
      "command": "agentstate-mcp",
      "env": {
        "AGENTSTATE_URL": "http://localhost:8080",
        "AGENTSTATE_NAMESPACE": "claude-memories",
        "AGENTSTATE_API_KEY": ""
      }
    }
  }
}
```

Restart Claude Desktop. You'll see 5 new memory tools available.

## Available Tools

| Tool | Description |
|------|-------------|
| `store_memory` | Store information persistently with optional tags |
| `retrieve_memory` | Retrieve a specific memory by ID |
| `search_memories` | Search memories by tag filters |
| `delete_memory` | Delete a memory by ID |
| `list_memories` | List all memories, optionally filtered by tags |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENTSTATE_URL` | `http://localhost:8080` | AgentState server URL |
| `AGENTSTATE_NAMESPACE` | `mcp-memory` | Namespace for memory storage |
| `AGENTSTATE_API_KEY` | _(none)_ | API key if auth is enabled |

## Example Interaction

Once configured, Claude can use persistent memory across conversations:

> **You:** Remember that my project uses Python 3.11 and PostgreSQL 15.
>
> **Claude:** I'll store that. *(calls store_memory with tags {"project": "current"})*
>
> *(New conversation)*
>
> **You:** What database am I using?
>
> **Claude:** *(calls search_memories with tags {"project": "current"})* You're using PostgreSQL 15.
