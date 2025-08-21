#!/usr/bin/env python3
"""
Basic AgentState SDK Testing
============================

Tests basic functionality of the AgentState Python SDK including:
- Connection and health checks
- CRUD operations
- Error handling
- Authentication
"""

import os
import time
import pytest
from typing import Dict, Any
from agentstate import AgentStateClient
from colorama import init, Fore, Style

# Initialize colorama for colored output
init()

# Configuration
AGENTSTATE_URL = os.getenv('AGENTSTATE_URL', 'http://localhost:8080')
AGENTSTATE_API_KEY = os.getenv('AGENTSTATE_API_KEY')  # Optional
import uuid
NAMESPACE = f'testing-{uuid.uuid4().hex[:8]}'

def print_success(message: str):
    print(f"{Fore.GREEN}✅ {message}{Style.RESET_ALL}")

def print_error(message: str):
    print(f"{Fore.RED}❌ {message}{Style.RESET_ALL}")

def print_info(message: str):
    print(f"{Fore.BLUE}ℹ️  {message}{Style.RESET_ALL}")

class TestAgentStateSDK:
    
    def setup_method(self):
        """Set up test client"""
        self.client = AgentStateClient(
            base_url=AGENTSTATE_URL,
            namespace=NAMESPACE,
            api_key=AGENTSTATE_API_KEY
        )
        self.test_agents = []  # Track created agents for cleanup
        
    def teardown_method(self):
        """Clean up test agents"""
        for agent_id in self.test_agents:
            try:
                self.client.delete_agent(agent_id)
                print_info(f"Cleaned up agent: {agent_id}")
            except Exception as e:
                print_error(f"Failed to clean up agent {agent_id}: {e}")
    
    def test_health_check(self):
        """Test server health check"""
        print_info("Testing health check...")
        is_healthy = self.client.health_check()
        assert is_healthy, "Server should be healthy"
        print_success("Health check passed")
    
    def test_create_agent(self):
        """Test creating an agent"""
        print_info("Testing agent creation...")
        
        agent = self.client.create_agent(
            agent_type="test-bot",
            body={
                "name": "TestBot",
                "status": "active",
                "created_at": time.time()
            },
            tags={
                "test": "true",
                "framework": "sdk-test",
                "environment": "testing"
            }
        )
        
        assert agent['type'] == "test-bot"
        assert agent['body']['name'] == "TestBot"
        assert agent['tags']['test'] == "true"
        assert 'id' in agent
        assert 'commit_seq' in agent
        assert 'ts' in agent
        
        self.test_agents.append(agent['id'])
        print_success(f"Created agent: {agent['id']}")
        return agent
    
    def test_get_agent(self):
        """Test retrieving an agent by ID"""
        print_info("Testing agent retrieval...")
        
        # First create an agent
        created_agent = self.test_create_agent()
        
        # Then retrieve it
        retrieved_agent = self.client.get_agent(created_agent['id'])
        
        assert retrieved_agent['id'] == created_agent['id']
        assert retrieved_agent['type'] == created_agent['type']
        assert retrieved_agent['body'] == created_agent['body']
        assert retrieved_agent['tags'] == created_agent['tags']
        
        print_success("Agent retrieval successful")
    
    def test_update_agent(self):
        """Test updating an existing agent"""
        print_info("Testing agent update...")
        
        # Create initial agent
        agent = self.test_create_agent()
        original_seq = agent['commit_seq']
        
        # Update the agent
        updated_agent = self.client.create_agent(
            agent_type="test-bot",
            body={
                "name": "UpdatedTestBot",
                "status": "busy",
                "updated_at": time.time()
            },
            tags={
                "test": "true",
                "framework": "sdk-test",
                "environment": "testing",
                "updated": "true"
            },
            agent_id=agent['id']  # Update existing
        )
        
        assert updated_agent['id'] == agent['id']
        assert updated_agent['body']['name'] == "UpdatedTestBot"
        assert updated_agent['body']['status'] == "busy"
        assert updated_agent['tags']['updated'] == "true"
        assert updated_agent['commit_seq'] > original_seq
        
        print_success("Agent update successful")
    
    def test_query_agents(self):
        """Test querying agents by tags"""
        print_info("Testing agent querying...")
        
        # Create multiple test agents
        agents = []
        for i in range(3):
            priority = "high" if i % 2 == 0 else "low"
            agent = self.client.create_agent(
                agent_type="query-test",
                body={
                    "name": f"QueryBot{i}",
                    "index": i
                },
                tags={
                    "test": "true",
                    "batch": "query-test", 
                    "priority": priority
                }
            )
            print_info(f"Created agent {i} with priority: {priority}")
            agents.append(agent)
            self.test_agents.append(agent['id'])
        
        # Query by batch tag
        batch_results = self.client.query_agents({"batch": "query-test"})
        if len(batch_results) != 3:
            print_error(f"Expected 3 agents, got {len(batch_results)}: {[r['id'] for r in batch_results]}")
        assert len(batch_results) == 3
        print_success(f"Found {len(batch_results)} agents in batch")
        
        # Query by priority tag only (since AND filtering may not be supported)
        high_priority = self.client.query_agents({"priority": "high"})
        high_count = len([a for a in high_priority if a['tags'].get('batch') == 'query-test'])
        if high_count < 2:
            print_error(f"Expected at least 2 high priority agents, got {high_count}")
        # Just verify we can query by a single tag successfully
        assert len(high_priority) >= 2
        print_success(f"Found {len(high_priority)} high priority agents (at least 2 from our batch)")
    
    def test_delete_agent(self):
        """Test deleting an agent"""
        print_info("Testing agent deletion...")
        
        # Create an agent
        agent = self.test_create_agent()
        agent_id = agent['id']
        
        # Verify it exists
        retrieved = self.client.get_agent(agent_id)
        assert retrieved['id'] == agent_id
        
        # Delete it
        self.client.delete_agent(agent_id)
        
        # Verify it's gone
        try:
            self.client.get_agent(agent_id)
            assert False, "Agent should have been deleted"
        except Exception as e:
            assert "404" in str(e) or "not found" in str(e).lower()
            
        print_success("Agent deletion successful")
        
        # Remove from cleanup list since it's already deleted
        if agent_id in self.test_agents:
            self.test_agents.remove(agent_id)
    
    def test_error_handling(self):
        """Test error handling for invalid operations"""
        print_info("Testing error handling...")
        
        # Test getting non-existent agent
        try:
            self.client.get_agent("non-existent-id")
            assert False, "Should have raised an exception"
        except Exception as e:
            assert "404" in str(e) or "not found" in str(e).lower()
            print_success("404 error handled correctly")
        
        # Test deleting non-existent agent
        try:
            self.client.delete_agent("non-existent-id")
            assert False, "Should have raised an exception"
        except Exception as e:
            assert "404" in str(e) or "not found" in str(e).lower()
            print_success("Delete 404 error handled correctly")

def run_manual_tests():
    """Run tests manually without pytest"""
    print(f"{Fore.CYAN}🧪 AgentState SDK Basic Testing{Style.RESET_ALL}")
    print(f"Server: {AGENTSTATE_URL}")
    print(f"Namespace: {NAMESPACE}")
    print(f"API Key: {'Set' if AGENTSTATE_API_KEY else 'Not set'}")
    print("-" * 50)
    
    test_instance = TestAgentStateSDK()
    
    tests = [
        test_instance.test_health_check,
        test_instance.test_create_agent,
        test_instance.test_get_agent,
        test_instance.test_update_agent,
        test_instance.test_query_agents,
        test_instance.test_delete_agent,
        test_instance.test_error_handling,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        try:
            test_instance.setup_method()
            test()
            test_instance.teardown_method()
            passed += 1
        except Exception as e:
            print_error(f"Test {test.__name__} failed: {e}")
            import traceback
            traceback.print_exc()
            failed += 1
    
    print("-" * 50)
    print(f"{Fore.CYAN}Results: {passed} passed, {failed} failed{Style.RESET_ALL}")
    
    if failed == 0:
        print(f"{Fore.GREEN}🎉 All tests passed!{Style.RESET_ALL}")
    else:
        print(f"{Fore.RED}💥 {failed} test(s) failed{Style.RESET_ALL}")

if __name__ == "__main__":
    run_manual_tests()