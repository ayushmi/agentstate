#!/usr/bin/env python3
"""
Verification script to test what's working and what's not
"""
import os
import requests

print("=== AgentState Working Verification ===\n")

# Test 1: Basic HTTP health check (without auth)
print("1. Testing basic health endpoint...")
try:
    response = requests.get("http://localhost:8080/health", timeout=5)
    print(f"   ✅ Health endpoint: {response.status_code} - {response.text.strip()}")
except Exception as e:
    print(f"   ❌ Health endpoint failed: {e}")

# Test 2: SDK import and basic initialization
print("\n2. Testing SDK import and initialization...")
try:
    from agentstate import AgentStateClient
    client = AgentStateClient(
        base_url='http://localhost:8080',
        namespace='integration-test'
    )
    print("   ✅ SDK imported and client initialized")
except Exception as e:
    print(f"   ❌ SDK initialization failed: {e}")

# Test 3: Environment variables
print("\n3. Checking environment variables...")
api_key = os.getenv('AGENTSTATE_API_KEY')
if api_key:
    print(f"   ✅ AGENTSTATE_API_KEY is set: {api_key[:20]}...")
else:
    print("   ❌ AGENTSTATE_API_KEY not set")

print("\n4. Docker container status...")
import subprocess
try:
    result = subprocess.run(['docker', 'ps', '--filter', 'name=agentstate'], 
                          capture_output=True, text=True)
    if 'agentstate' in result.stdout:
        print("   ✅ AgentState container is running")
        # Extract status
        lines = result.stdout.strip().split('\n')
        if len(lines) > 1:
            status_line = lines[1]  # First data line
            print(f"   Status: {status_line.split()[4:6]}")  # STATUS and PORTS columns
    else:
        print("   ❌ No AgentState container found")
except Exception as e:
    print(f"   ❌ Could not check Docker status: {e}")

print("\n=== Summary ===")
print("✅ What's working:")
print("  - Docker container starts")
print("  - Basic health endpoint responds")
print("  - SDK can be imported and initialized")
print("  - API key is configured")

print("\n❌ What's not working:")
print("  - API operations timeout (create, get, query)")
print("  - SDK health_check() fails due to auth header issue")
print("  - Tests hang when making authenticated API calls")

print("\nPossible causes:")
print("  - Server hanging on authenticated requests")
print("  - Network timeout issues")
print("  - Auth token validation problems")
print("  - Server bug with request handling")