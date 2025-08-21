#!/usr/bin/env python3
"""Debug test to see what's happening with the SDK"""

import traceback
from agentstate import AgentStateClient

# Test basic functionality
client = AgentStateClient("http://localhost:8080", "testing")

print("Testing health check...")
try:
    health = client.health_check()
    print(f"Health check result: {health}")
except Exception as e:
    print(f"Health check error: {e}")
    traceback.print_exc()

print("\nTesting agent creation...")
try:
    agent = client.create_agent(
        agent_type="test-bot",
        body={"name": "TestBot", "status": "active"},
        tags={"test": "true"}
    )
    print(f"Agent created: {agent}")
except Exception as e:
    print(f"Agent creation error: {e}")
    traceback.print_exc()