#!/usr/bin/env python3
"""
Test AgentState Integration (No OpenAI Required)
=============================================

Tests basic AgentState integration without requiring OpenAI API key.
"""

import os
import time
from agentstate import AgentStateClient


def test_agentstate_connection():
    """Test basic AgentState connection and operations"""
    print("🧪 Testing AgentState Connection...")
    
    # Initialize AgentState client
    client = AgentStateClient(
        base_url=os.getenv('AGENTSTATE_URL', 'http://localhost:8080'),
        namespace='integration-test',
        api_key=os.getenv('AGENTSTATE_API_KEY')
    )
    
    # Add timeout
    client.session.timeout = 5
    
    # Test health check
    print("Testing health check...")
    try:
        health = client.health_check()
        print(f"✅ Health check: {health}")
    except Exception as e:
        print(f"❌ Health check failed: {e}")
        raise
    
    # Test agent creation
    print("Testing agent creation...")
    print("About to call create_agent...")
    agent_id = f"test-agent-{int(time.time())}"
    
    agent = client.create_agent(
        agent_type="test-agent",
        body={
            "name": f"Test Agent {agent_id}",
            "memory": {"messages": []},
            "created_at": time.time()
        },
        tags={
            "framework": "test",
            "type": "integration-test"
        },
        agent_id=agent_id
    )
    print(f"✅ Created agent: {agent['id']}")
    
    # Test agent retrieval
    print("Testing agent retrieval...")
    retrieved_agent = client.get_agent(agent_id)
    print(f"✅ Retrieved agent: {retrieved_agent['id']}")
    
    # Test agent update
    print("Testing agent update...")
    updated_body = retrieved_agent['body'].copy()
    updated_body['memory']['messages'].append({
        "type": "test",
        "content": "Test message",
        "timestamp": time.time()
    })
    
    updated_agent = client.create_agent(
        agent_type="test-agent",
        body=updated_body,
        tags=retrieved_agent['tags'],
        agent_id=agent_id
    )
    print(f"✅ Updated agent: {updated_agent['id']}")
    
    # Test agent query
    print("Testing agent query...")
    agents = client.query_agents({"framework": "test"})
    print(f"✅ Found {len(agents)} test agents")
    
    # Cleanup
    print("Cleaning up...")
    client.delete_agent(agent_id)
    print(f"✅ Deleted agent: {agent_id}")
    
    print("\n🎉 All AgentState integration tests passed!")
    return True


if __name__ == "__main__":
    try:
        test_agentstate_connection()
    except Exception as e:
        print(f"❌ Integration test failed: {e}")
        import traceback
        traceback.print_exc()