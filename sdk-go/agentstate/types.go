// Package agentstate provides a Go client for the AgentState HTTP API.
package agentstate

import "time"

// Tags is a map of string key-value pairs used for querying objects.
type Tags map[string]string

// Cause records why a state change happened. All fields are optional.
type Cause struct {
	Actor   string `json:"actor,omitempty"`   // agent ID making this change
	Trigger string `json:"trigger,omitempty"` // commit hash that triggered this change
	Note    string `json:"note,omitempty"`    // human-readable reason
}

// Object is a persisted AgentState object returned by the server.
type Object struct {
	ID         string         `json:"id"`
	Namespace  string         `json:"ns"`
	Type       string         `json:"type"`
	Body       map[string]any `json:"body"`
	Tags       Tags           `json:"tags"`
	TTLSeconds *uint64        `json:"ttl_seconds,omitempty"`
	Parents    []string       `json:"parents,omitempty"`
	Commit     string         `json:"commit"`
	Timestamp  time.Time      `json:"ts"`
	CommitSeq  uint64         `json:"commit_seq"`
	Cause      *Cause         `json:"cause,omitempty"`
}

// PutRequest is the payload for creating or updating an object.
type PutRequest struct {
	Type       string         `json:"type"`
	Body       map[string]any `json:"body"`
	Tags       Tags           `json:"tags,omitempty"`
	TTLSeconds *uint64        `json:"ttl_seconds,omitempty"`
	ID         string         `json:"id,omitempty"`
	Parents    []string       `json:"parents,omitempty"`
	Cause      *Cause         `json:"cause,omitempty"`
}

// WatchEvent is a parsed Server-Sent Event from the watch stream.
type WatchEvent struct {
	Type      string  `json:"type"`              // "put" or "delete"
	Obj       *Object `json:"obj,omitempty"`     // present for "put" events
	ID        string  `json:"id,omitempty"`      // present for "delete" events
	CommitSeq uint64  `json:"commit_seq,omitempty"`
}

// WatchFunc is called for each SSE event received from the watch stream.
// Return false to stop watching.
type WatchFunc func(event WatchEvent) bool
