"""
Demonstration of AgentState runtime invariant assertions (Layer 2).

Run with:
    docker run -p 8080:8080 ayushmi/agentstate:latest
    pip install -r requirements.txt
    python setup_invariants.py
"""

import requests
from agentstate import AgentStateClient

BASE_URL = "http://localhost:8080"
NS = "verify-demo"

client = AgentStateClient(BASE_URL, NS)


def main():
    print("=== AgentState Invariant Demo ===\n")

    # 1. Set a namespace invariant
    print("1. Setting invariant on namespace:", NS)
    spec = client.set_invariant(NS, [
        {"field": "body.status",   "required": True, "one_of": ["active", "idle", "stopped"]},
        {"field": "body.score",    "gte": 0.0, "lte": 1.0},
        {"field": "body.agent_id", "required": True, "type": "string"},
    ])
    print("   Stored spec:", spec)

    # 2. Write a valid object — should succeed
    print("\n2. Writing a valid object...")
    obj = client.create_agent("agent", {
        "agent_id": "agent-001",
        "status": "active",
        "score": 0.85,
    }, tags={"team": "platform"})
    print("   Created:", obj["id"], "commit:", obj["commit"][:8])

    # 3. Upsert the same object — should succeed
    print("\n3. Updating status to 'idle'...")
    obj2 = client.create_agent("agent", {
        "agent_id": "agent-001",
        "status": "idle",
        "score": 0.72,
    }, tags={"team": "platform"}, agent_id=obj["id"])
    print("   Updated:", obj2["id"], "prev_commit:", obj2.get("prev_commit", "n/a")[:8])

    # 4. Try an invalid write — status not in allowed list
    print("\n4. Attempting write with invalid status 'unknown'...")
    try:
        client.create_agent("agent", {
            "agent_id": "agent-002",
            "status": "unknown",   # not in one_of
            "score": 0.5,
        })
        print("   ERROR: should have been rejected!")
    except requests.HTTPError as e:
        resp = e.response.json()
        print("   Correctly rejected (409):")
        for v in resp.get("violations", []):
            print("    -", v)

    # 5. Try an invalid score
    print("\n5. Attempting write with score out of range (1.5)...")
    try:
        client.create_agent("agent", {
            "agent_id": "agent-003",
            "status": "active",
            "score": 1.5,   # > 1.0
        })
        print("   ERROR: should have been rejected!")
    except requests.HTTPError as e:
        resp = e.response.json()
        print("   Correctly rejected (409):")
        for v in resp.get("violations", []):
            print("    -", v)

    # 6. Chain verify
    print("\n6. Verifying hash chain...")
    r = requests.get(f"{BASE_URL}/admin/namespaces/{NS}/chain-verify")
    data = r.json()
    if data["ok"]:
        print(f"   Chain intact — {data['objects_checked']} objects checked, 0 breaks")
    else:
        print("   Chain BROKEN:", data["breaks"])

    print("\nDone.")


if __name__ == "__main__":
    main()
