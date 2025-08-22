#!/usr/bin/env python3
"""Minimal test to identify where it hangs"""

print("Starting minimal test...")

try:
    print("1. Importing agentstate...")
    from agentstate import AgentStateClient
    print("   ✅ Import successful")
    
    print("2. Creating client...")
    client = AgentStateClient(
        base_url='http://localhost:8080',
        namespace='integration-test'
    )
    print("   ✅ Client created")
    
    print("3. Setting timeout...")
    client.session.timeout = 3
    print("   ✅ Timeout set")
    
    print("4. Testing health check...")
    import sys
    sys.stdout.flush()  # Force output before potential hang
    
    health = client.health_check()
    print(f"   ✅ Health check result: {health}")
    
    if health:
        print("5. Testing simple create_agent...")
        agent = client.create_agent(
            agent_type="test",
            body={"test": True}
        )
        print(f"   ✅ Agent created: {agent['id']}")
        
        print("6. Cleanup...")
        client.delete_agent(agent['id'])
        print("   ✅ Agent deleted")
    
    print("All tests passed!")
    
except Exception as e:
    print(f"❌ Error: {e}")
    import traceback
    traceback.print_exc()