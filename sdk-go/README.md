# AgentState Go SDK

Go client for [AgentState](https://github.com/ayushmi/agentstate) — "Firebase for AI Agents".

**stdlib only** — no external dependencies.

## Install

```bash
go get github.com/ayushmi/agentstate/sdk-go
```

## Quick Start

```go
import agentstate "github.com/ayushmi/agentstate/sdk-go/agentstate"

client := agentstate.NewClient("http://localhost:8080", "production",
    agentstate.WithAPIKey(os.Getenv("AGENTSTATE_API_KEY")),
)

// Create or update an agent
obj, err := client.Put(agentstate.PutRequest{
    Type: "agent",
    Body: map[string]any{"status": "active", "name": "worker-1"},
    Tags: agentstate.Tags{"env": "production", "team": "platform"},
    Cause: &agentstate.Cause{
        Actor: "scheduler",
        Note:  "initial deployment",
    },
})

// Get latest version
obj, err = client.Get(obj.ID, time.Time{})

// Get state as of 10 minutes ago (time-travel)
old, err := client.Get(obj.ID, time.Now().Add(-10*time.Minute))

// Query by tags
agents, err := client.Query(agentstate.Tags{"env": "production", "status": "active"})

// Watch for real-time changes
err = client.Watch(func(ev agentstate.WatchEvent) bool {
    fmt.Printf("[%s] %s\n", ev.Type, ev.ID)
    return true // return false to stop
})

// Delete
err = client.Delete(obj.ID)
```

## API

| Method | Description |
|--------|-------------|
| `NewClient(baseURL, namespace, opts...)` | Create a client |
| `WithAPIKey(key)` | Option: set Bearer token |
| `WithHTTPClient(hc)` | Option: set custom http.Client |
| `Put(PutRequest)` | Create or update an object |
| `Get(id, atTime)` | Get object; pass `time.Time{}` for latest, non-zero for time-travel |
| `Query(tags)` | List objects matching all tags |
| `Delete(id)` | Delete an object |
| `Watch(fn)` | Stream SSE events; return false from fn to stop |
| `Health()` | Returns true if server is healthy |

## Types

```go
type PutRequest struct {
    Type       string
    Body       map[string]any
    Tags       Tags
    ID         string      // optional stable ID for upserts
    Parents    []string    // optional parent commit hashes
    Cause      *Cause      // optional provenance
}

type Cause struct {
    Actor   string // agent ID making this change
    Trigger string // commit hash that triggered this
    Note    string // human-readable reason
}

type Object struct {
    ID, Namespace, Type string
    Body                map[string]any
    Tags                Tags
    Commit              string
    CommitSeq           uint64
    Timestamp           time.Time
    Cause               *Cause
}
```

## Requirements

- Go 1.21+
- AgentState server ([quickstart](https://github.com/ayushmi/agentstate#quick-start))
