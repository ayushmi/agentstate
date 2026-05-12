package agentstate

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Client is the AgentState HTTP client for Go.
//
// Example:
//
//	client := agentstate.NewClient("http://localhost:8080", "production")
//	obj, err := client.Put(agentstate.PutRequest{
//	    Type: "agent",
//	    Body: map[string]any{"status": "active"},
//	    Tags: agentstate.Tags{"env": "prod"},
//	})
type Client struct {
	baseURL   string
	namespace string
	apiKey    string
	http      *http.Client
}

// ClientOption is a functional option for configuring a Client.
type ClientOption func(*Client)

// WithAPIKey sets the API key sent as "Authorization: Bearer <key>" on every request.
func WithAPIKey(key string) ClientOption {
	return func(c *Client) { c.apiKey = key }
}

// WithHTTPClient replaces the default http.Client (e.g. to configure TLS or timeouts).
func WithHTTPClient(hc *http.Client) ClientOption {
	return func(c *Client) { c.http = hc }
}

// NewClient creates a new AgentState client for the given server URL and namespace.
//
//	client := agentstate.NewClient("http://localhost:8080", "production")
//	client := agentstate.NewClient("http://localhost:8080", "production",
//	    agentstate.WithAPIKey(os.Getenv("AGENTSTATE_API_KEY")),
//	)
func NewClient(baseURL, namespace string, opts ...ClientOption) *Client {
	c := &Client{
		baseURL:   strings.TrimRight(baseURL, "/"),
		namespace: namespace,
		http:      &http.Client{Timeout: 30 * time.Second},
	}
	for _, o := range opts {
		o(c)
	}
	return c
}

func (c *Client) newRequest(method, path string, body any) (*http.Request, error) {
	var buf io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, fmt.Errorf("agentstate: marshal request: %w", err)
		}
		buf = bytes.NewReader(b)
	}
	req, err := http.NewRequest(method, c.baseURL+path, buf)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("User-Agent", "agentstate-go-sdk/0.1.0")
	if c.apiKey != "" {
		req.Header.Set("Authorization", "Bearer "+c.apiKey)
	}
	return req, nil
}

func (c *Client) do(req *http.Request, out any) error {
	resp, err := c.http.Do(req)
	if err != nil {
		return fmt.Errorf("agentstate: %w", err)
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("agentstate: read response: %w", err)
	}
	if resp.StatusCode >= 300 {
		return fmt.Errorf("agentstate: status %d: %s", resp.StatusCode, data)
	}
	if out != nil {
		if err := json.Unmarshal(data, out); err != nil {
			return fmt.Errorf("agentstate: decode response: %w", err)
		}
	}
	return nil
}

// Put creates or updates an object in the client's namespace.
//
//	obj, err := client.Put(agentstate.PutRequest{
//	    Type: "agent",
//	    Body: map[string]any{"status": "active"},
//	    Tags: agentstate.Tags{"env": "prod"},
//	    Cause: &agentstate.Cause{Actor: "scheduler", Note: "initial setup"},
//	})
func (c *Client) Put(req PutRequest) (*Object, error) {
	hreq, err := c.newRequest("POST", fmt.Sprintf("/v1/%s/objects", c.namespace), req)
	if err != nil {
		return nil, err
	}
	var obj Object
	return &obj, c.do(hreq, &obj)
}

// Get retrieves an object by ID.
// Pass a zero time.Time{} to get the latest version.
// Pass a non-zero time to use the server's time-travel feature (?at=<RFC3339>).
//
//	// Latest version
//	obj, err := client.Get("01JXYZ...", time.Time{})
//
//	// State as of 10 minutes ago
//	obj, err := client.Get("01JXYZ...", time.Now().Add(-10*time.Minute))
func (c *Client) Get(id string, atTime time.Time) (*Object, error) {
	path := fmt.Sprintf("/v1/%s/objects/%s", c.namespace, id)
	if !atTime.IsZero() {
		path += "?at=" + atTime.UTC().Format(time.RFC3339)
	}
	hreq, err := c.newRequest("GET", path, nil)
	if err != nil {
		return nil, err
	}
	var obj Object
	return &obj, c.do(hreq, &obj)
}

// Query lists objects in the namespace matching all provided tags.
// Pass nil or empty Tags to return all objects.
//
//	agents, err := client.Query(agentstate.Tags{"env": "prod", "status": "active"})
func (c *Client) Query(tags Tags) ([]Object, error) {
	payload := map[string]any{}
	if len(tags) > 0 {
		payload["tag_filter"] = tags
	}
	hreq, err := c.newRequest("POST", fmt.Sprintf("/v1/%s/query", c.namespace), payload)
	if err != nil {
		return nil, err
	}
	var objs []Object
	return objs, c.do(hreq, &objs)
}

// Delete removes an object by ID.
func (c *Client) Delete(id string) error {
	hreq, err := c.newRequest("DELETE", fmt.Sprintf("/v1/%s/objects/%s", c.namespace, id), nil)
	if err != nil {
		return err
	}
	return c.do(hreq, nil)
}

// Watch subscribes to the Server-Sent Events watch stream for the client's namespace.
// fn is called for each event. Return false from fn to stop watching.
// Watch blocks until fn returns false, the stream ends, or an error occurs.
//
//	err := client.Watch(func(ev agentstate.WatchEvent) bool {
//	    fmt.Printf("event: %s id: %s\n", ev.Type, ev.ID)
//	    return true // keep watching
//	})
func (c *Client) Watch(fn WatchFunc) error {
	url := fmt.Sprintf("%s/v1/%s/watch", c.baseURL, c.namespace)
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "text/event-stream")
	req.Header.Set("Cache-Control", "no-cache")
	if c.apiKey != "" {
		req.Header.Set("Authorization", "Bearer "+c.apiKey)
	}

	resp, err := c.http.Do(req)
	if err != nil {
		return fmt.Errorf("agentstate: watch connect: %w", err)
	}
	defer resp.Body.Close()

	scanner := bufio.NewScanner(resp.Body)
	var dataLine string
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "data:") {
			dataLine = strings.TrimSpace(strings.TrimPrefix(line, "data:"))
		} else if line == "" && dataLine != "" {
			var ev WatchEvent
			if err := json.Unmarshal([]byte(dataLine), &ev); err == nil {
				if !fn(ev) {
					return nil
				}
			}
			dataLine = ""
		}
	}
	return scanner.Err()
}

// Health returns true if the AgentState server is reachable and healthy.
func (c *Client) Health() bool {
	resp, err := c.http.Get(c.baseURL + "/health")
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode == 200
}

// SetInvariant configures a namespace invariant (enforced before every write).
// rules is a slice of rule maps, e.g.:
//
//	[]map[string]any{
//	    {"field": "body.status", "required": true},
//	    {"field": "body.score", "gte": 0, "lte": 1},
//	}
func (c *Client) SetInvariant(ns string, rules []map[string]any) (map[string]any, error) {
	payload := map[string]any{"rules": rules}
	hreq, err := c.newRequest("POST", fmt.Sprintf("/admin/namespaces/%s/invariants", ns), payload)
	if err != nil {
		return nil, err
	}
	var result map[string]any
	return result, c.do(hreq, &result)
}

// GetInvariant retrieves the current invariant spec for a namespace.
// Returns nil, nil if no invariant is set.
func (c *Client) GetInvariant(ns string) (map[string]any, error) {
	hreq, err := c.newRequest("GET", fmt.Sprintf("/admin/namespaces/%s/invariants", ns), nil)
	if err != nil {
		return nil, err
	}
	var result map[string]any
	if err := c.do(hreq, &result); err != nil {
		// 404 means no invariant set — treat as nil, nil
		if strings.Contains(err.Error(), "status 404") {
			return nil, nil
		}
		return nil, err
	}
	return result, nil
}

// ── Claim Verification ────────────────────────────────────────────────────────

// ClaimSubmitResult is returned by SubmitClaim — it contains both the stored
// claim and its formal proof artifact.
type ClaimSubmitResult struct {
	Claim map[string]any `json:"claim"`
	Proof map[string]any `json:"proof"`
}

// SubmitClaim submits a claim for formal verification and returns the claim
// together with its proof artifact.
//
//	result, err := client.SubmitClaim("production", map[string]any{
//	    "domain": "healthcare/v1",
//	    "template": "drug_safety",
//	    "assertion": map[string]any{
//	        "predicate": "safe_to_prescribe",
//	        "subject":   map[string]any{"patient_id": "p-001"},
//	        "object":    map[string]any{"drug": "amoxicillin"},
//	    },
//	    "premises": []map[string]any{
//	        {"kind":"source","id":"allergy-db-v3","role":"allergy_clearance"},
//	    },
//	})
func (c *Client) SubmitClaim(ns string, req map[string]any) (*ClaimSubmitResult, error) {
	hreq, err := c.newRequest("POST", fmt.Sprintf("/admin/namespaces/%s/claims", ns), req)
	if err != nil {
		return nil, err
	}
	var result ClaimSubmitResult
	return &result, c.do(hreq, &result)
}

// GetClaim retrieves a stored claim by ID.
func (c *Client) GetClaim(ns, claimID string) (map[string]any, error) {
	hreq, err := c.newRequest("GET", fmt.Sprintf("/admin/namespaces/%s/claims/%s", ns, claimID), nil)
	if err != nil {
		return nil, err
	}
	var result map[string]any
	return result, c.do(hreq, &result)
}

// GetProof retrieves the formal proof artifact for a claim.
func (c *Client) GetProof(ns, claimID string) (map[string]any, error) {
	hreq, err := c.newRequest("GET", fmt.Sprintf("/admin/namespaces/%s/claims/%s/proof", ns, claimID), nil)
	if err != nil {
		return nil, err
	}
	var result map[string]any
	return result, c.do(hreq, &result)
}

// ListClaims returns all claims stored in a namespace.
func (c *Client) ListClaims(ns string) ([]map[string]any, error) {
	hreq, err := c.newRequest("GET", fmt.Sprintf("/admin/namespaces/%s/claims", ns), nil)
	if err != nil {
		return nil, err
	}
	var result []map[string]any
	return result, c.do(hreq, &result)
}

// ChallengeClaim submits a challenge against a claim's proof.
// challengedStep is the 0-based step index to challenge, or -1 to challenge the entire proof.
func (c *Client) ChallengeClaim(ns, claimID, reason string, challengedStep int, counterEvidence []string) (map[string]any, error) {
	payload := map[string]any{
		"reason":           reason,
		"counter_evidence": counterEvidence,
	}
	if challengedStep >= 0 {
		payload["challenged_step"] = challengedStep
	}
	hreq, err := c.newRequest("POST", fmt.Sprintf("/admin/namespaces/%s/claims/%s/challenge", ns, claimID), payload)
	if err != nil {
		return nil, err
	}
	var result map[string]any
	return result, c.do(hreq, &result)
}

// ListDomains returns all available domain packs.
func (c *Client) ListDomains() ([]map[string]any, error) {
	hreq, err := c.newRequest("GET", "/admin/domains", nil)
	if err != nil {
		return nil, err
	}
	var result []map[string]any
	return result, c.do(hreq, &result)
}
