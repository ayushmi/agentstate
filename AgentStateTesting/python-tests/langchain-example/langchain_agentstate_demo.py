#!/usr/bin/env python3
"""
LangChain + AgentState Integration Example
==========================================

Demonstrates how to use AgentState as persistent storage for LangChain agents.
This example shows:
- Custom LangChain memory backed by AgentState
- Multi-agent conversation state management
- Agent coordination through shared state
"""

import os
import time
from typing import Dict, Any, List, Optional
from agentstate import AgentStateClient
from langchain.memory import BaseChatMessageHistory
from langchain.schema import BaseMessage, HumanMessage, AIMessage
from langchain_openai import ChatOpenAI
from langchain.agents import AgentExecutor, create_openai_functions_agent
from langchain.tools import Tool
from langchain.prompts import ChatPromptTemplate, MessagesPlaceholder
from pydantic import BaseModel


class AgentStateMemory(BaseChatMessageHistory):
    """LangChain memory implementation using AgentState as backend"""
    
    def __init__(self, agent_id: str, agentstate_client: AgentStateClient):
        self.agent_id = agent_id
        self.client = agentstate_client
        self._ensure_agent_exists()
    
    def _ensure_agent_exists(self):
        """Ensure the agent exists in AgentState"""
        try:
            self.client.get_agent(self.agent_id)
        except:
            # Create agent if it doesn't exist
            self.client.create_agent(
                agent_type="langchain-agent",
                body={
                    "name": f"LangChain Agent {self.agent_id}",
                    "memory": {"messages": []},
                    "created_at": time.time()
                },
                tags={
                    "framework": "langchain",
                    "type": "chat-agent"
                },
                agent_id=self.agent_id
            )
    
    @property 
    def messages(self) -> List[BaseMessage]:
        """Get messages from AgentState"""
        agent = self.client.get_agent(self.agent_id)
        messages_data = agent['body'].get('memory', {}).get('messages', [])
        
        messages = []
        for msg_data in messages_data:
            if msg_data['type'] == 'human':
                messages.append(HumanMessage(content=msg_data['content']))
            elif msg_data['type'] == 'ai':
                messages.append(AIMessage(content=msg_data['content']))
        
        return messages
    
    def add_message(self, message: BaseMessage) -> None:
        """Add a message to AgentState"""
        # Get current agent state
        agent = self.client.get_agent(self.agent_id)
        current_messages = agent['body'].get('memory', {}).get('messages', [])
        
        # Add new message
        if isinstance(message, HumanMessage):
            msg_data = {"type": "human", "content": message.content}
        elif isinstance(message, AIMessage):
            msg_data = {"type": "ai", "content": message.content}
        else:
            msg_data = {"type": "system", "content": str(message.content)}
        
        current_messages.append(msg_data)
        
        # Update agent in AgentState
        updated_body = agent['body'].copy()
        updated_body['memory']['messages'] = current_messages
        updated_body['last_message_at'] = time.time()
        
        self.client.create_agent(
            agent_type="langchain-agent",
            body=updated_body,
            tags=agent['tags'],
            agent_id=self.agent_id
        )
    
    def clear(self) -> None:
        """Clear all messages"""
        agent = self.client.get_agent(self.agent_id)
        updated_body = agent['body'].copy()
        updated_body['memory']['messages'] = []
        updated_body['cleared_at'] = time.time()
        
        self.client.create_agent(
            agent_type="langchain-agent",
            body=updated_body,
            tags=agent['tags'],
            agent_id=self.agent_id
        )


def create_calculator_tool():
    """Create a simple calculator tool"""
    def calculate(expression: str) -> str:
        """Safely evaluate mathematical expressions"""
        try:
            # Basic safety check
            allowed_chars = set('0123456789+-*/(). ')
            if not all(c in allowed_chars for c in expression):
                return "Error: Invalid characters in expression"
            
            result = eval(expression)
            return f"Result: {result}"
        except Exception as e:
            return f"Error: {str(e)}"
    
    return Tool(
        name="Calculator",
        description="Useful for mathematical calculations. Input should be a mathematical expression.",
        func=calculate
    )


def create_agent_coordination_tool(agentstate_client: AgentStateClient):
    """Create a tool for agents to coordinate with each other"""
    def coordinate_with_agents(message: str) -> str:
        """Send a coordination message to all agents in the system"""
        try:
            # Get all langchain agents
            agents = agentstate_client.query_agents({"framework": "langchain"})
            
            # Store coordination message
            coord_msg = agentstate_client.create_agent(
                agent_type="coordination-message",
                body={
                    "message": message,
                    "timestamp": time.time(),
                    "sender": "coordination-tool"
                },
                tags={
                    "type": "coordination",
                    "framework": "langchain"
                }
            )
            
            return f"Coordination message sent to {len(agents)} agents. Message ID: {coord_msg['id']}"
        except Exception as e:
            return f"Error sending coordination message: {str(e)}"
    
    return Tool(
        name="AgentCoordination",
        description="Send messages to coordinate with other agents in the system",
        func=coordinate_with_agents
    )


class LangChainAgentStateDemo:
    """Demo class for LangChain + AgentState integration"""
    
    def __init__(self):
        # Initialize AgentState client
        self.agentstate = AgentStateClient(
            base_url=os.getenv('AGENTSTATE_URL', 'http://localhost:8080'),
            namespace='langchain-demo',
            api_key=os.getenv('AGENTSTATE_API_KEY')
        )
        
        # Initialize OpenAI (requires OPENAI_API_KEY environment variable)
        self.llm = ChatOpenAI(
            model="gpt-3.5-turbo",
            temperature=0.7
        )
        
        # Create tools
        self.tools = [
            create_calculator_tool(),
            create_agent_coordination_tool(self.agentstate)
        ]
        
        self.agents = {}
    
    def create_agent(self, agent_id: str, system_prompt: str) -> AgentExecutor:
        """Create a LangChain agent with AgentState memory"""
        
        # Create AgentState-backed memory
        memory = AgentStateMemory(agent_id, self.agentstate)
        
        # Create prompt template
        prompt = ChatPromptTemplate.from_messages([
            ("system", system_prompt),
            MessagesPlaceholder(variable_name="chat_history"),
            ("human", "{input}"),
            MessagesPlaceholder(variable_name="agent_scratchpad")
        ])
        
        # Create agent
        agent = create_openai_functions_agent(self.llm, self.tools, prompt)
        agent_executor = AgentExecutor(
            agent=agent,
            tools=self.tools,
            verbose=True,
            memory=memory
        )
        
        self.agents[agent_id] = agent_executor
        return agent_executor
    
    def run_multi_agent_demo(self):
        """Run a demo with multiple coordinating agents"""
        print("🤖 LangChain + AgentState Multi-Agent Demo")
        print("=" * 50)
        
        # Create two specialized agents
        math_agent = self.create_agent(
            "math-specialist",
            "You are a mathematics specialist. Help with calculations and math problems. "
            "Use the Calculator tool for computations. You can also coordinate with other agents."
        )
        
        coord_agent = self.create_agent(
            "coordinator",
            "You are a coordinator agent. Your job is to help organize tasks and coordinate "
            "between different agents. Use the AgentCoordination tool to communicate with other agents."
        )
        
        print("\n📊 Math Agent solving a problem:")
        math_response = math_agent.invoke({
            "input": "Calculate the area of a circle with radius 5, then tell other agents about this calculation"
        })
        print(f"Math Agent: {math_response['output']}")
        
        print("\n🎯 Coordinator organizing tasks:")
        coord_response = coord_agent.invoke({
            "input": "Check what coordination messages have been sent and organize a summary"
        })
        print(f"Coordinator: {coord_response['output']}")
        
        print("\n📈 Checking AgentState for stored data:")
        # Query all agents and coordination messages
        agents = self.agentstate.query_agents({"framework": "langchain"})
        coord_messages = self.agentstate.query_agents({"type": "coordination"})
        
        print(f"Total agents in system: {len(agents)}")
        print(f"Coordination messages: {len(coord_messages)}")
        
        for agent in agents:
            if agent['type'] == 'langchain-agent':
                print(f"  Agent {agent['id']}: {len(agent['body'].get('memory', {}).get('messages', []))} messages")
    
    def cleanup(self):
        """Clean up test agents"""
        try:
            agents = self.agentstate.query_agents({"framework": "langchain"})
            for agent in agents:
                self.agentstate.delete_agent(agent['id'])
            print(f"🧹 Cleaned up {len(agents)} test agents")
        except Exception as e:
            print(f"❌ Cleanup error: {e}")


def main():
    """Run the LangChain + AgentState demo"""
    if not os.getenv('OPENAI_API_KEY'):
        print("❌ Please set OPENAI_API_KEY environment variable")
        return
    
    demo = LangChainAgentStateDemo()
    
    try:
        demo.run_multi_agent_demo()
    finally:
        demo.cleanup()


if __name__ == "__main__":
    main()