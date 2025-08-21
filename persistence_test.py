#!/usr/bin/env python3
"""
💾 AgentState Persistence and WAL Test
=====================================

Tests data persistence, WAL functionality, and crash recovery.
"""

import requests
import json
import time
import subprocess
import docker
import sys
from typing import Dict, Any, List

class PersistenceTestClient:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.docker_client = docker.from_env()
    
    def health_check(self) -> bool:
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            return response.status_code == 200 and response.text.strip() == "ok"
        except:
            return False
    
    def create_agent(self, agent_type: str, body: Dict[str, Any], tags: Dict[str, str] = None) -> Dict[str, Any]:
        payload = {"type": agent_type, "body": body, "tags": tags or {}}
        response = self.session.post(f"{self.base_url}/v1/persistence-test/objects", json=payload)
        response.raise_for_status()
        return response.json()
    
    def get_agent(self, agent_id: str) -> Dict[str, Any]:
        response = self.session.get(f"{self.base_url}/v1/persistence-test/objects/{agent_id}")
        response.raise_for_status()
        return response.json()
    
    def query_agents(self, tags: Dict[str, str] = None) -> List[Dict[str, Any]]:
        query = {"tags": tags} if tags else {}
        response = self.session.post(f"{self.base_url}/v1/persistence-test/query", json=query)
        response.raise_for_status()
        return response.json()
    
    def restart_container(self, container_name: str = "agentstate-test-persistent"):
        """Restart the AgentState container to test persistence"""
        try:
            container = self.docker_client.containers.get(container_name)
            print("🔄 Stopping AgentState container...")
            container.stop(timeout=10)
            
            print("🚀 Starting AgentState container...")
            container.start()
            
            # Wait for container to be ready
            print("⏳ Waiting for container to be ready...")
            max_retries = 30
            for i in range(max_retries):
                if self.health_check():
                    print(f"✅ Container ready after {i + 1} attempts")
                    return True
                time.sleep(1)
            
            print("❌ Container failed to start within timeout")
            return False
            
        except docker.errors.NotFound:
            print(f"❌ Container {container_name} not found")
            return False
        except Exception as e:
            print(f"❌ Error restarting container: {e}")
            return False

def test_persistence_after_restart():
    """Test that data persists after container restart"""
    print("💾 Testing Persistence After Restart")
    print("=" * 40)
    
    client = PersistenceTestClient()
    
    # Check if server is running
    if not client.health_check():
        print("❌ AgentState server is not running")
        return False
    
    try:
        # Create test data
        print("📝 Creating test agents...")
        test_agents = []
        
        for i in range(5):
            agent = client.create_agent(
                "persistent-agent",
                {
                    "name": f"PersistentAgent{i}",
                    "created_at": time.time(),
                    "data": f"Important data {i}",
                    "counter": i * 10
                },
                {
                    "test": "persistence",
                    "batch": "before-restart",
                    "index": str(i)
                }
            )
            test_agents.append(agent)
            print(f"  ✅ Created agent {i}: {agent['id']}")
        
        # Verify data exists before restart
        print("🔍 Verifying data before restart...")
        agents_before = client.query_agents({"batch": "before-restart"})
        assert len(agents_before) == 5, f"Expected 5 agents, found {len(agents_before)}"
        print(f"  ✅ Found {len(agents_before)} agents before restart")
        
        # Store agent IDs for post-restart verification
        agent_ids = [agent["id"] for agent in test_agents]
        
        # Restart container
        print("\n🔄 Restarting container to test persistence...")
        if not client.restart_container():
            return False
        
        # Verify data persists after restart
        print("🔍 Verifying data after restart...")
        agents_after = client.query_agents({"batch": "before-restart"})
        print(f"  📊 Found {len(agents_after)} agents after restart")
        
        if len(agents_after) != 5:
            print(f"❌ Expected 5 agents after restart, found {len(agents_after)}")
            return False
        
        # Verify specific agents by ID
        print("🔍 Verifying individual agents by ID...")
        for i, agent_id in enumerate(agent_ids):
            try:
                agent = client.get_agent(agent_id)
                expected_name = f"PersistentAgent{i}"
                if agent["body"]["name"] != expected_name:
                    print(f"❌ Agent {agent_id} name mismatch: expected {expected_name}, got {agent['body']['name']}")
                    return False
                print(f"  ✅ Agent {i} ({agent_id}): {agent['body']['name']}")
            except Exception as e:
                print(f"❌ Failed to retrieve agent {agent_id}: {e}")
                return False
        
        # Test updating existing data after restart
        print("✏️  Testing updates after restart...")
        updated_agent = client.create_agent(
            "persistent-agent",
            {
                "name": "PersistentAgent0",
                "created_at": time.time(),
                "data": "Updated data after restart",
                "counter": 999,
                "updated_after_restart": True
            },
            {
                "test": "persistence", 
                "batch": "before-restart",
                "index": "0"
            },
            agent_ids[0]  # Update first agent
        )
        
        # Verify update worked
        retrieved = client.get_agent(agent_ids[0])
        if retrieved["body"]["counter"] != 999:
            print(f"❌ Update failed: expected counter 999, got {retrieved['body']['counter']}")
            return False
        
        if not retrieved["body"].get("updated_after_restart"):
            print("❌ Update flag not found")
            return False
        
        print("  ✅ Successfully updated agent after restart")
        
        print("\n🎉 Persistence test passed!")
        return True
        
    except Exception as e:
        print(f"❌ Persistence test failed: {e}")
        return False

def test_wal_recovery():
    """Test Write-Ahead Log recovery functionality"""
    print("\n📝 Testing WAL Recovery")
    print("=" * 30)
    
    client = PersistenceTestClient()
    
    try:
        # Create agents to populate WAL
        print("📝 Creating agents to populate WAL...")
        wal_agents = []
        
        for i in range(10):
            agent = client.create_agent(
                "wal-test-agent",
                {
                    "name": f"WALAgent{i}",
                    "sequence": i,
                    "timestamp": time.time()
                },
                {
                    "test": "wal-recovery",
                    "sequence": str(i)
                }
            )
            wal_agents.append(agent)
            # Small delay to ensure different timestamps
            time.sleep(0.1)
        
        print(f"  ✅ Created {len(wal_agents)} agents for WAL test")
        
        # Verify all agents exist
        wal_query = client.query_agents({"test": "wal-recovery"})
        assert len(wal_query) == 10, f"Expected 10 WAL agents, found {len(wal_query)}"
        
        # Simulate crash and recovery by restarting
        print("🔄 Simulating crash recovery via restart...")
        if not client.restart_container():
            return False
        
        # Verify WAL recovery
        print("🔍 Verifying WAL recovery...")
        recovered_agents = client.query_agents({"test": "wal-recovery"})
        
        if len(recovered_agents) != 10:
            print(f"❌ WAL recovery failed: expected 10 agents, found {len(recovered_agents)}")
            return False
        
        # Verify sequence integrity
        sequences = sorted([agent["body"]["sequence"] for agent in recovered_agents])
        expected_sequences = list(range(10))
        
        if sequences != expected_sequences:
            print(f"❌ Sequence integrity check failed: expected {expected_sequences}, got {sequences}")
            return False
        
        print("  ✅ All agents recovered from WAL with correct sequence")
        print("🎉 WAL recovery test passed!")
        return True
        
    except Exception as e:
        print(f"❌ WAL recovery test failed: {e}")
        return False

def main():
    print("💾 AgentState Persistence and WAL Testing")
    print("=" * 50)
    
    success = True
    
    # Test persistence
    if not test_persistence_after_restart():
        success = False
    
    # Test WAL recovery
    if not test_wal_recovery():
        success = False
    
    if success:
        print("\n🎉 All persistence tests passed!")
        print("✅ Data survives restarts")
        print("✅ WAL recovery works correctly")
        print("✅ Updates work after restart")
        return 0
    else:
        print("\n❌ Some persistence tests failed")
        return 1

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n⏹️  Test interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Test suite error: {e}")
        sys.exit(1)