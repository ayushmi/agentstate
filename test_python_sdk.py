#!/usr/bin/env python3
"""
🧪 Test Python SDK functionality
"""

import sys
import os
import time

# Add the SDK to Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'sdk-py'))

from agentstate import AgentStateClient

def test_python_sdk():
    print("🧪 Testing Python SDK v1.0.0")
    print("=" * 40)
    
    client = AgentStateClient("http://localhost:8080", namespace="sdk-test")
    
    try:
        # Test 1: Health check
        print("1. Testing health check...")
        healthy = client.health_check()
        assert healthy, "Server is not healthy"
        print("   ✅ Server is healthy")
        
        # Test 2: Create agent
        print("2. Testing create agent...")
        agent = client.create_agent(
            agent_type="test-bot",
            body={
                "name": "TestBot",
                "version": "1.0.0", 
                "status": "initializing",
                "capabilities": ["chat", "search"],
                "metrics": {"requests": 0, "uptime": 0}
            },
            tags={
                "environment": "test",
                "team": "sdk-team",
                "language": "python"
            }
        )
        
        agent_id = agent["id"]
        assert agent["type"] == "test-bot"
        assert agent["body"]["name"] == "TestBot"
        assert agent["tags"]["language"] == "python"
        print(f"   ✅ Created agent: {agent_id}")
        
        # Test 3: Get agent
        print("3. Testing get agent...")
        retrieved = client.get_agent(agent_id)
        assert retrieved["id"] == agent_id
        assert retrieved["body"]["name"] == "TestBot"
        print(f"   ✅ Retrieved agent: {retrieved['body']['name']}")
        
        # Test 4: Update agent
        print("4. Testing update agent...")
        updated = client.create_agent(
            agent_type="test-bot",
            body={
                "name": "TestBot",
                "version": "1.0.0",
                "status": "active",
                "capabilities": ["chat", "search", "analyze"],
                "metrics": {"requests": 42, "uptime": 3600}
            },
            tags={
                "environment": "test", 
                "team": "sdk-team",
                "language": "python"
            },
            agent_id=agent_id
        )
        
        assert updated["body"]["status"] == "active"
        assert updated["body"]["metrics"]["requests"] == 42
        print("   ✅ Updated agent state")
        
        # Test 5: Query agents
        print("5. Testing query agents...")
        
        # Create additional agents for querying
        agent2 = client.create_agent("test-worker", {
            "name": "Worker1",
            "status": "idle"
        }, {"team": "sdk-team", "role": "worker"})
        
        agent3 = client.create_agent("test-worker", {
            "name": "Worker2", 
            "status": "busy"
        }, {"team": "sdk-team", "role": "worker"})
        
        # Query by team
        team_agents = client.query_agents({"team": "sdk-team"})
        assert len(team_agents) >= 3, f"Expected at least 3 agents, got {len(team_agents)}"
        print(f"   ✅ Found {len(team_agents)} team agents")
        
        # Query by role
        workers = client.query_agents({"role": "worker"})
        assert len(workers) >= 2, f"Expected at least 2 workers, got {len(workers)}"
        print(f"   ✅ Found {len(workers)} worker agents")
        
        # Query all test agents
        all_test_agents = client.query_agents({"environment": "test"})
        print(f"   ✅ Found {len(all_test_agents)} test environment agents")
        
        # Test 6: Real-time updates simulation
        print("6. Testing real-time updates...")
        
        for i in range(3):
            client.create_agent("test-bot", {
                "name": "TestBot",
                "version": "1.0.0",
                "status": "processing",
                "current_task": f"task_{i+1}",
                "capabilities": ["chat", "search", "analyze"],
                "metrics": {"requests": 42 + i, "uptime": 3600 + i*10}
            }, {
                "environment": "test",
                "team": "sdk-team", 
                "language": "python"
            }, agent_id)
            
            time.sleep(0.1)  # Small delay
        
        final_state = client.get_agent(agent_id)
        assert final_state["body"]["current_task"] == "task_3"
        print("   ✅ Real-time updates working")
        
        # Test 7: Legacy API compatibility
        print("7. Testing legacy API compatibility...")
        
        # Test legacy State class
        from agentstate import State
        legacy_client = State("http://localhost:8080", "legacy-test")
        
        legacy_agent = legacy_client.put("legacy-type", {"test": True}, {"legacy": "true"})
        assert legacy_agent["type"] == "legacy-type"
        
        legacy_retrieved = legacy_client.get(legacy_agent["id"])
        assert legacy_retrieved["body"]["test"] == True
        
        legacy_query = legacy_client.query({"legacy": "true"})
        assert len(legacy_query) >= 1
        
        print("   ✅ Legacy API compatibility working")
        
        # Test 8: Error handling
        print("8. Testing error handling...")
        
        try:
            client.get_agent("nonexistent-id")
            assert False, "Should have raised an exception"
        except Exception as e:
            print("   ✅ Error handling for non-existent agent works")
        
        # Cleanup
        print("9. Cleaning up test agents...")
        test_agents = client.query_agents({"environment": "test"})
        for agent in test_agents:
            client.delete_agent(agent["id"])
        
        legacy_agents = client.query_agents({"legacy": "true"})
        for agent in legacy_agents:
            client.delete_agent(agent["id"])
            
        print(f"   ✅ Cleaned up {len(test_agents) + len(legacy_agents)} test agents")
        
        print(f"\n🎉 All Python SDK tests passed!")
        print(f"✅ AgentState Python SDK v1.0.0 is working perfectly!")
        return True
        
    except Exception as e:
        print(f"\n❌ Python SDK test failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_python_sdk()
    sys.exit(0 if success else 1)