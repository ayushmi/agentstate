#!/usr/bin/env python3
"""
🤖 AgentState Python SDK Example
=================================

This example demonstrates how to use AgentState as "Firebase for AI Agents"
- Store agent state persistently 
- Query agents by tags
- Subscribe to real-time updates
- Manage agent lifecycle

Install: pip install requests
Usage: python python_example.py
"""

import requests
import json
import time
from datetime import datetime
from typing import Dict, Any, List, Optional

class AgentStateClient:
    """Simple AgentState HTTP client"""
    
    def __init__(self, base_url: str = "http://localhost:8080", namespace: str = "production"):
        self.base_url = base_url.rstrip('/')
        self.namespace = namespace
        self.session = requests.Session()
    
    def create_agent(self, agent_type: str, body: Dict[str, Any], tags: Dict[str, str] = None, agent_id: str = None) -> Dict[str, Any]:
        """Create or update an agent"""
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
        """Get agent by ID"""
        response = self.session.get(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()
        return response.json()
    
    def query_agents(self, tags: Dict[str, str] = None, type_filter: str = None) -> List[Dict[str, Any]]:
        """Query agents by tags or type"""
        query = {}
        if tags:
            query["tags"] = tags
        
        response = self.session.post(
            f"{self.base_url}/v1/{self.namespace}/query",
            json=query,
            headers={"Content-Type": "application/json"}
        )
        response.raise_for_status()
        agents = response.json()
        
        # Filter by type if specified
        if type_filter:
            agents = [a for a in agents if a.get("type") == type_filter]
            
        return agents
    
    def delete_agent(self, agent_id: str) -> None:
        """Delete an agent"""
        response = self.session.delete(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()

def main():
    """Example usage demonstrating AgentState as Firebase for AI Agents"""
    
    print("🤖 AgentState Python Example")
    print("============================")
    
    # Initialize client
    client = AgentStateClient(namespace="my-ai-app")
    
    try:
        # 1. Create a chatbot agent
        print("\n📝 Creating chatbot agent...")
        chatbot = client.create_agent(
            agent_type="chatbot",
            body={
                "name": "CustomerSupportBot",
                "model": "gpt-4",
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
                "model": "gpt-4",
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
        chatbots = client.query_agents(type_filter="chatbot")
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
        
    except requests.exceptions.RequestException as e:
        print(f"❌ API Error: {e}")
        print("Make sure AgentState server is running on http://localhost:8080")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    main()