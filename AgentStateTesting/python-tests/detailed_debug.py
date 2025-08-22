#!/usr/bin/env python3
"""Detailed debug to see exactly what the SDK is doing"""

print("Starting detailed debug...")

try:
    print("1. Manual requests test:")
    import requests
    response = requests.get("http://localhost:8080/health", timeout=5)
    print(f"   Status: {response.status_code}")
    print(f"   Text: '{response.text}'")
    print(f"   Text.strip(): '{response.text.strip()}'")
    print(f"   Length: {len(response.text)}")
    print(f"   Equals 'ok'? {response.text.strip() == 'ok'}")
    
    print("\n2. AgentState client test:")
    from agentstate import AgentStateClient
    
    client = AgentStateClient(
        base_url='http://localhost:8080',
        namespace='debug-test'
    )
    
    print(f"   Client base_url: {client.base_url}")
    print(f"   Expected URL: {client.base_url}/health")
    
    # Test the same request the SDK makes
    print("   Making SDK-style request...")
    sdk_response = client.session.get(f"{client.base_url}/health", timeout=5)
    print(f"   SDK Status: {sdk_response.status_code}")
    print(f"   SDK Text: '{sdk_response.text}'")
    print(f"   SDK Text.strip(): '{sdk_response.text.strip()}'")
    print(f"   SDK Check: {sdk_response.status_code == 200 and sdk_response.text.strip() == 'ok'}")
    
    # Now call the actual health_check method
    print("   Calling health_check method...")
    health = client.health_check()
    print(f"   Health result: {health}")
    
except Exception as e:
    print(f"❌ Error: {e}")
    import traceback
    traceback.print_exc()