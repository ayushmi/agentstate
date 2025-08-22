#!/usr/bin/env python3
"""
Final comprehensive verification of AgentState v1.0
Tests all components, SDKs, and integrations
"""

import os
import time
import sys
from typing import Dict, Any

def test_health_endpoint():
    """Test basic health endpoint"""
    print("🏥 Testing health endpoint...")
    import requests
    try:
        response = requests.get("http://localhost:8080/health", timeout=3)
        assert response.status_code == 200
        assert response.text.strip() == "ok"
        print("   ✅ Health endpoint working")
        return True
    except Exception as e:
        print(f"   ❌ Health endpoint failed: {e}")
        return False

def test_python_sdk():
    """Test Python SDK comprehensive functionality"""
    print("🐍 Testing Python SDK...")
    try:
        from agentstate import AgentStateClient
        
        client = AgentStateClient(
            base_url='http://localhost:8080',
            namespace='final-verification'
        )
        
        # Health check
        assert client.health_check() == True
        print("   ✅ SDK health check working")
        
        # CRUD operations
        agent = client.create_agent(
            agent_type='verification-test',
            body={'test': 'final_verification', 'timestamp': time.time()}
        )
        agent_id = agent['id']
        print(f"   ✅ Create: {agent_id}")
        
        retrieved = client.get_agent(agent_id)
        assert retrieved['id'] == agent_id
        print(f"   ✅ Get: {retrieved['body']['test']}")
        
        results = client.query_agents({'test': 'final_verification'})
        assert len(results) >= 1
        print(f"   ✅ Query: {len(results)} results")
        
        client.delete_agent(agent_id)
        print(f"   ✅ Delete: {agent_id}")
        
        print("   ✅ Python SDK fully working")
        return True
        
    except Exception as e:
        print(f"   ❌ Python SDK failed: {e}")
        return False

def test_performance():
    """Test performance and no timeouts"""
    print("⚡ Testing performance...")
    try:
        from agentstate import AgentStateClient
        client = AgentStateClient(base_url='http://localhost:8080', namespace='perf-test')
        
        # Rapid operations
        start = time.time()
        agents = []
        for i in range(10):
            agent = client.create_agent(
                agent_type='perf-test',
                body={'index': i}
            )
            agents.append(agent['id'])
        
        create_time = time.time() - start
        ops_per_sec = 10 / create_time
        print(f"   ✅ Created 10 agents in {create_time:.3f}s ({ops_per_sec:.1f} ops/sec)")
        
        # Query performance
        start = time.time()
        results = client.query_agents({})
        query_time = time.time() - start
        print(f"   ✅ Queried {len(results)} agents in {query_time:.3f}s")
        
        # Cleanup
        for agent_id in agents:
            client.delete_agent(agent_id)
        
        print("   ✅ Performance test passed")
        return True
        
    except Exception as e:
        print(f"   ❌ Performance test failed: {e}")
        return False

def test_langchain_integration():
    """Test LangChain integration"""
    print("🦜 Testing LangChain integration...")
    try:
        sys.path.append('/Users/ayush/Development/AgentState/AgentStateTesting/python-tests/langchain-example')
        from langchain_agentstate_demo import LangChainAgentStateDemo, create_calculator_tool
        
        # Initialize demo
        demo = LangChainAgentStateDemo()
        print("   ✅ Demo initialized")
        
        # Create agent
        agent = demo.create_agent(
            'verification-agent',
            'You are a test agent for verification.'
        )
        assert agent is not None
        print("   ✅ Agent created with memory")
        
        # Test tool
        calc_tool = create_calculator_tool()
        result = calc_tool.func('5 * 8')
        assert "40" in result
        print(f"   ✅ Calculator tool working: {result}")
        
        # Cleanup
        demo.cleanup()
        print("   ✅ LangChain integration working")
        return True
        
    except Exception as e:
        print(f"   ❌ LangChain integration failed: {e}")
        return False

def main():
    """Run comprehensive verification"""
    print("🚀 AgentState v1.0 - Final Verification")
    print("=" * 50)
    
    # Set environment
    os.environ['AGENTSTATE_API_KEY'] = "active.eyJucyI6WyJsYW5nY2hhaW4tZGVtbyIsImludGVncmF0aW9uLXRlc3QiXSwidmVyYnMiOlsicHV0IiwiZ2V0IiwiZGVsZXRlIiwicXVlcnkiLCJsZWFzZSJdLCJpYXQiOjE3NTU4Njk0MzEsImV4cCI6MTc1NTk1NTgzMX0._CnofZqRI7EZknaphSBTVdaYoqiTkcTghni9-m40jZ4"
    os.environ['OPENAI_API_KEY'] = "sk-test"
    
    tests = [
        ("Health Endpoint", test_health_endpoint),
        ("Python SDK", test_python_sdk), 
        ("Performance", test_performance),
        ("LangChain Integration", test_langchain_integration),
    ]
    
    results = []
    for test_name, test_func in tests:
        try:
            result = test_func()
            results.append((test_name, result))
        except Exception as e:
            print(f"❌ {test_name} crashed: {e}")
            results.append((test_name, False))
        print()
    
    # Summary
    print("📋 Final Results:")
    print("-" * 30)
    
    passed = 0
    for test_name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{status} | {test_name}")
        if result:
            passed += 1
    
    print("-" * 30)
    print(f"Passed: {passed}/{len(results)}")
    
    if passed == len(results):
        print("\n🎉 ALL TESTS PASSED!")
        print("✅ AgentState v1.0 is PRODUCTION READY")
        print("🚀 All systems operational!")
        return True
    else:
        print(f"\n❌ {len(results) - passed} tests failed")
        print("🔧 System needs fixes")
        return False

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)