package agentstate_test

import (
	"fmt"
	"testing"
	"time"

	agentstate "github.com/ayushmi/agentstate/sdk-go/agentstate"
)

// TestNewClient verifies the client can be constructed without panicking.
func TestNewClient(t *testing.T) {
	client := agentstate.NewClient("http://localhost:8080", "test",
		agentstate.WithAPIKey("test-key"),
	)
	if client == nil {
		t.Fatal("expected non-nil client")
	}
}

// Example shows typical usage of the Go SDK.
// This is an example function — it won't run without a live server,
// but it documents the API and is checked by `go build`.
func Example() {
	client := agentstate.NewClient("http://localhost:8080", "go-example")

	// Create an object
	obj, err := client.Put(agentstate.PutRequest{
		Type: "agent",
		Body: map[string]any{"status": "active", "name": "worker-1"},
		Tags: agentstate.Tags{"env": "production"},
		Cause: &agentstate.Cause{
			Actor: "scheduler",
			Note:  "initial deployment",
		},
	})
	if err != nil {
		panic(err)
	}
	fmt.Println("Created:", obj.ID)

	// Retrieve latest version
	got, err := client.Get(obj.ID, time.Time{})
	if err != nil {
		panic(err)
	}
	fmt.Println("Status:", got.Body["status"])

	// Retrieve as of 1 hour ago (time-travel)
	_, _ = client.Get(obj.ID, time.Now().Add(-time.Hour))

	// Query by tag
	agents, err := client.Query(agentstate.Tags{"env": "production"})
	if err != nil {
		panic(err)
	}
	fmt.Println("Found agents:", len(agents))

	// Delete
	_ = client.Delete(obj.ID)
}
