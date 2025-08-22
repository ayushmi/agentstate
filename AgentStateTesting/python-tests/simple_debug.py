#!/usr/bin/env python3
"""Debug AgentState connection step by step"""

import requests
import time

print("1. Testing direct HTTP call...")
try:
    response = requests.get("http://localhost:8080/health", timeout=5)
    print(f"   Status: {response.status_code}")
    print(f"   Content: {response.text}")
except Exception as e:
    print(f"   Error: {e}")

print("\n2. Testing AgentState import...")
try:
    from agentstate import AgentStateClient
    print("   Import successful")
except Exception as e:
    print(f"   Import error: {e}")

print("\n3. Testing AgentState client initialization...")
try:
    client = AgentStateClient(
        base_url='http://localhost:8080',
        namespace='debug-test'
    )
    print("   Client initialized")
except Exception as e:
    print(f"   Client init error: {e}")

print("\n4. Testing health check with timeout...")
try:
    client.session.timeout = 3
    health = client.health_check()
    print(f"   Health check result: {health}")
except Exception as e:
    print(f"   Health check error: {e}")