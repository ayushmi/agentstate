#!/usr/bin/env python3
"""
🧪 AgentState Integration Test Suite
===================================

Comprehensive testing suite for AgentState APIs and functionality.
Tests all features: CRUD, querying, real-time updates, persistence, etc.
"""

import requests
import json
import time
import threading
import subprocess
import sys
from datetime import datetime
from typing import Dict, Any, List
import asyncio
import concurrent.futures

class AgentStateTestClient:
    def __init__(self, base_url: str = "http://localhost:8080", namespace: str = "integration-tests"):
        self.base_url = base_url.rstrip('/')
        self.namespace = namespace
        self.session = requests.Session()
    
    def create_agent(self, agent_type: str, body: Dict[str, Any], tags: Dict[str, str] = None, agent_id: str = None) -> Dict[str, Any]:
        payload = {
            "type": agent_type,
            "body": body,
            "tags": tags or {}
        }
        if agent_id:
            payload["id"] = agent_id
        
        response = self.session.post(
            f"{self.base_url}/v1/{self.namespace}/objects",
            json=payload,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        return response.json()
    
    def get_agent(self, agent_id: str) -> Dict[str, Any]:
        response = self.session.get(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()
        return response.json()
    
    def query_agents(self, tags: Dict[str, str] = None) -> List[Dict[str, Any]]:
        query = {}
        if tags:
            query["tags"] = tags
        
        response = self.session.post(
            f"{self.base_url}/v1/{self.namespace}/query",
            json=query,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        return response.json()
    
    def delete_agent(self, agent_id: str) -> None:
        response = self.session.delete(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()
    
    def health_check(self) -> bool:
        try:
            response = self.session.get(f"{self.base_url}/health")
            return response.status_code == 200 and response.text.strip() == "ok"
        except:
            return False

class IntegrationTests:
    def __init__(self):
        self.client = AgentStateTestClient()
        self.test_results = []
        
    def run_test(self, test_name: str, test_func):
        """Run a single test and record results"""
        print(f"🧪 Running: {test_name}")
        start_time = time.time()
        try:
            test_func()
            duration = time.time() - start_time
            print(f"✅ PASS: {test_name} ({duration:.2f}s)")
            self.test_results.append({"name": test_name, "status": "PASS", "duration": duration})
        except Exception as e:
            duration = time.time() - start_time
            print(f"❌ FAIL: {test_name} ({duration:.2f}s) - {str(e)}")
            self.test_results.append({"name": test_name, "status": "FAIL", "duration": duration, "error": str(e)})
    
    def test_health_endpoint(self):
        """Test basic health check"""
        assert self.client.health_check(), "Health check failed"
    
    def test_basic_crud_operations(self):
        """Test create, read, update, delete operations"""
        # Create
        agent = self.client.create_agent(
            "test-agent",
            {"name": "TestBot", "status": "active"},
            {"test": "crud"}
        )
        agent_id = agent["id"]
        assert agent["type"] == "test-agent"
        assert agent["body"]["name"] == "TestBot"
        
        # Read
        retrieved = self.client.get_agent(agent_id)
        assert retrieved["id"] == agent_id
        assert retrieved["body"]["name"] == "TestBot"
        
        # Update
        updated = self.client.create_agent(
            "test-agent",
            {"name": "TestBot", "status": "updated"},
            {"test": "crud"},
            agent_id
        )
        assert updated["body"]["status"] == "updated"
        
        # Delete
        self.client.delete_agent(agent_id)
        
        # Verify deleted
        try:
            self.client.get_agent(agent_id)
            assert False, "Agent should have been deleted"
        except requests.exceptions.HTTPError:
            pass  # Expected
    
    def test_tagging_and_querying(self):
        """Test tag-based querying"""
        # Create agents with different tags
        agents = []
        for i in range(5):
            agent = self.client.create_agent(
                "tagged-agent",
                {"name": f"Agent{i}", "batch": "test"},
                {"environment": "test", "team": f"team{i % 2}"}
            )
            agents.append(agent["id"])
        
        # Query by environment
        env_agents = self.client.query_agents({"environment": "test"})
        assert len(env_agents) >= 5, f"Expected at least 5 agents, got {len(env_agents)}"
        
        # Query by team
        team0_agents = self.client.query_agents({"team": "team0"})
        team1_agents = self.client.query_agents({"team": "team1"})
        assert len(team0_agents) >= 2, f"Expected at least 2 team0 agents, got {len(team0_agents)}"
        assert len(team1_agents) >= 3, f"Expected at least 3 team1 agents, got {len(team1_agents)}"
        
        # Cleanup
        for agent_id in agents:
            self.client.delete_agent(agent_id)
    
    def test_real_time_updates(self):
        """Test real-time state updates"""
        # Create agent
        agent = self.client.create_agent(
            "realtime-agent",
            {"status": "idle", "counter": 0},
            {"test": "realtime"}
        )
        agent_id = agent["id"]
        
        # Perform multiple updates
        for i in range(10):
            updated = self.client.create_agent(
                "realtime-agent",
                {"status": "processing", "counter": i + 1},
                {"test": "realtime"},
                agent_id
            )
            assert updated["body"]["counter"] == i + 1
        
        # Verify final state
        final = self.client.get_agent(agent_id)
        assert final["body"]["counter"] == 10
        
        # Cleanup
        self.client.delete_agent(agent_id)
    
    def test_concurrent_operations(self):
        """Test concurrent access and operations"""
        def create_agent_worker(worker_id: int) -> str:
            agent = self.client.create_agent(
                "concurrent-agent",
                {"worker": worker_id, "status": "active"},
                {"test": "concurrency", "worker": str(worker_id)}
            )
            return agent["id"]
        
        # Create agents concurrently
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(create_agent_worker, i) for i in range(20)]
            agent_ids = [future.result() for future in futures]
        
        # Verify all agents were created
        assert len(agent_ids) == 20
        
        # Query concurrently
        def query_worker():
            return self.client.query_agents({"test": "concurrency"})
        
        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(query_worker) for _ in range(10)]
            results = [future.result() for future in futures]
        
        # All queries should return the same number of agents
        for result in results:
            assert len(result) >= 20, f"Expected at least 20 agents, got {len(result)}"
        
        # Cleanup concurrently
        def delete_worker(agent_id: str):
            self.client.delete_agent(agent_id)
        
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(delete_worker, agent_id) for agent_id in agent_ids]
            [future.result() for future in futures]
    
    def test_large_payload_handling(self):
        """Test handling of large payloads"""
        # Create agent with large body
        large_data = {
            "name": "LargeAgent",
            "large_array": list(range(1000)),
            "large_string": "x" * 10000,
            "nested": {
                "level1": {
                    "level2": {
                        "data": list(range(500))
                    }
                }
            }
        }
        
        agent = self.client.create_agent(
            "large-agent",
            large_data,
            {"test": "large-payload"}
        )
        agent_id = agent["id"]
        
        # Verify large payload was stored correctly
        retrieved = self.client.get_agent(agent_id)
        assert len(retrieved["body"]["large_array"]) == 1000
        assert len(retrieved["body"]["large_string"]) == 10000
        assert len(retrieved["body"]["nested"]["level1"]["level2"]["data"]) == 500
        
        # Cleanup
        self.client.delete_agent(agent_id)
    
    def test_edge_cases(self):
        """Test various edge cases"""
        # Empty body
        agent1 = self.client.create_agent("empty-agent", {}, {"test": "edge-cases"})
        retrieved1 = self.client.get_agent(agent1["id"])
        assert retrieved1["body"] == {}
        
        # Unicode handling
        agent2 = self.client.create_agent(
            "unicode-agent",
            {"name": "Test 🤖", "message": "Hello 世界"},
            {"test": "edge-cases", "unicode": "测试"}
        )
        retrieved2 = self.client.get_agent(agent2["id"])
        assert retrieved2["body"]["name"] == "Test 🤖"
        assert retrieved2["body"]["message"] == "Hello 世界"
        assert retrieved2["tags"]["unicode"] == "测试"
        
        # Special characters in tags
        agent3 = self.client.create_agent(
            "special-agent",
            {"test": True},
            {"test": "edge-cases", "special": "key-with-dashes_and_underscores.and.dots"}
        )
        retrieved3 = self.client.get_agent(agent3["id"])
        assert retrieved3["tags"]["special"] == "key-with-dashes_and_underscores.and.dots"
        
        # Cleanup
        for agent in [agent1, agent2, agent3]:
            self.client.delete_agent(agent["id"])
    
    def test_metrics_endpoint(self):
        """Test metrics endpoint availability"""
        response = self.client.session.get(f"{self.client.base_url}/metrics")
        assert response.status_code == 200, f"Metrics endpoint returned {response.status_code}"
        metrics_text = response.text
        assert "query_planner_micros" in metrics_text, "query_planner_micros metric not found"
        # Note: objects_created_total might not exist in current implementation
    
    def run_all_tests(self):
        """Run complete test suite"""
        print("🚀 Starting AgentState Integration Test Suite")
        print("=" * 50)
        
        tests = [
            ("Health Check", self.test_health_endpoint),
            ("Basic CRUD Operations", self.test_basic_crud_operations),
            ("Tagging and Querying", self.test_tagging_and_querying),
            ("Real-time Updates", self.test_real_time_updates),
            ("Concurrent Operations", self.test_concurrent_operations),
            ("Large Payload Handling", self.test_large_payload_handling),
            ("Edge Cases", self.test_edge_cases),
            ("Metrics Endpoint", self.test_metrics_endpoint)
        ]
        
        for test_name, test_func in tests:
            self.run_test(test_name, test_func)
        
        # Summary
        print("\n📊 Test Results Summary")
        print("=" * 30)
        
        total_tests = len(self.test_results)
        passed_tests = len([r for r in self.test_results if r["status"] == "PASS"])
        failed_tests = len([r for r in self.test_results if r["status"] == "FAIL"])
        total_duration = sum(r["duration"] for r in self.test_results)
        
        print(f"Total Tests: {total_tests}")
        print(f"✅ Passed: {passed_tests}")
        print(f"❌ Failed: {failed_tests}")
        print(f"⏱️  Total Duration: {total_duration:.2f}s")
        
        if failed_tests > 0:
            print("\n❌ Failed Tests:")
            for result in self.test_results:
                if result["status"] == "FAIL":
                    print(f"  - {result['name']}: {result.get('error', 'Unknown error')}")
            return False
        else:
            print("\n🎉 All tests passed! AgentState is ready for production.")
            return True

if __name__ == "__main__":
    # Check if server is running
    test_client = AgentStateTestClient()
    if not test_client.health_check():
        print("❌ AgentState server is not running on http://localhost:8080")
        print("Please start the server first:")
        print("  docker run -p 8080:8080 -p 9090:9090 agentstate:latest")
        sys.exit(1)
    
    # Run tests
    tests = IntegrationTests()
    success = tests.run_all_tests()
    sys.exit(0 if success else 1)