#!/usr/bin/env python3
"""
🤖 AgentState Python SDK Example
=================================

This example demonstrates how to use AgentState as "Firebase for AI Agents"
- Store agent state persistently 
- Query agents by tags
- Subscribe to real-time updates
- Manage agent lifecycle

Install: pip install agentstate
Usage: python python_example.py
"""

import time
from datetime import datetime
from agentstate import AgentStateClient

def main():
    """Example usage demonstrating AgentState as Firebase for AI Agents"""
    
    print("🤖 AgentState Python Example")
    print("============================")
    
    # Initialize client  
    client = AgentStateClient(base_url="http://localhost:8080", namespace="my-ai-app")
    
    try:
        # 1. Create a chatbot agent
        print("\n📝 Creating chatbot agent...")
        chatbot = client.create_agent(
            agent_type="chatbot",
            body={
                "name": "CustomerSupportBot",
                "model": "llm-model-v1",
                "temperature": 0.7,
                "status": "active",
                "current_conversation": None,
                "stats": {
                    "conversations_handled": 0,
                    "avg_response_time": 0
                }
            },
            tags={
                "environment": "production",
                "team": "customer-support",
                "version": "1.2.0"
            }
        )
        chatbot_id = chatbot["id"]
        print(f"✅ Created chatbot: {chatbot_id}")
        
        # 2. Create a data processing agent
        print("\n📊 Creating data processor agent...")
        processor = client.create_agent(
            agent_type="processor",
            body={
                "name": "DataPipelineProcessor",
                "status": "idle",
                "current_job": None,
                "queue_size": 0,
                "processed_today": 0
            },
            tags={
                "environment": "production",
                "team": "data",
                "capability": "batch-processing"
            }
        )
        processor_id = processor["id"]
        print(f"✅ Created processor: {processor_id}")
        
        # 3. Update chatbot state (simulate handling a conversation)
        print("\n💬 Updating chatbot state (simulating conversation)...")
        updated_chatbot = client.create_agent(
            agent_type="chatbot",
            agent_id=chatbot_id,  # Update existing agent
            body={
                "name": "CustomerSupportBot",
                "model": "llm-model-v1",
                "temperature": 0.7,
                "status": "busy",
                "current_conversation": "conv_12345",
                "stats": {
                    "conversations_handled": 1,
                    "avg_response_time": 1.2
                }
            },
            tags={
                "environment": "production",
                "team": "customer-support", 
                "version": "1.2.0"
            }
        )
        print("✅ Updated chatbot state")
        
        # 4. Query all production agents
        print("\n🔍 Querying all production agents...")
        production_agents = client.query_agents(tags={"environment": "production"})
        print(f"✅ Found {len(production_agents)} production agents:")
        for agent in production_agents:
            print(f"  - {agent['body']['name']} ({agent['type']}) - {agent['body']['status']}")
        
        # 5. Query specific agent types
        print("\n🤖 Querying chatbot agents...")
        all_agents = client.query_agents()
        chatbots = [a for a in all_agents if a.get("type") == "chatbot"]
        print(f"✅ Found {len(chatbots)} chatbot agents")
        
        # 6. Demonstrate real-time state management
        print("\n⚡ Demonstrating real-time state updates...")
        for i in range(3):
            # Update processor with new job
            client.create_agent(
                agent_type="processor",
                agent_id=processor_id,
                body={
                    "name": "DataPipelineProcessor",
                    "status": "processing",
                    "current_job": f"job_{i+1}",
                    "queue_size": 5 - i,
                    "processed_today": i + 1
                },
                tags={
                    "environment": "production",
                    "team": "data",
                    "capability": "batch-processing"
                }
            )
            
            # Get current state
            current_state = client.get_agent(processor_id)
            job = current_state["body"]["current_job"]
            queue = current_state["body"]["queue_size"]
            print(f"  Processing {job}, queue size: {queue}")
            
            time.sleep(1)  # Simulate processing time
        
        print("✅ Real-time updates complete")
        
        # 7. Clean up (optional)
        print("\n🧹 Cleaning up test agents...")
        client.delete_agent(chatbot_id)
        client.delete_agent(processor_id)
        print("✅ Cleanup complete")
        
        print(f"\n🎉 Example complete! AgentState makes it easy to:")
        print("   📦 Store agent state persistently")
        print("   🔍 Query agents by tags and type") 
        print("   ⚡ Update agent state in real-time")
        print("   🔄 Manage agent lifecycle")
        print(f"\nReady to integrate into your AI application! 🚀")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        print("Make sure AgentState server is running on http://localhost:8080")

if __name__ == "__main__":
    main()