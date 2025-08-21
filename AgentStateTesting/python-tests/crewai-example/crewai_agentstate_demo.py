#!/usr/bin/env python3
"""
CrewAI + AgentState Integration Example
=======================================

Demonstrates how to use AgentState with CrewAI for persistent agent state management.
This example shows:
- CrewAI agents with AgentState persistent storage
- Task coordination through shared state
- Agent memory and context persistence
"""

import os
import time
from typing import Dict, Any, List, Optional
from agentstate import AgentStateClient
from crewai import Agent, Task, Crew, Process
from langchain_openai import ChatOpenAI
from langchain.tools import tool


class AgentStateManager:
    """Manages AgentState integration for CrewAI agents"""
    
    def __init__(self, base_url: str = None, namespace: str = "crewai-demo", api_key: str = None):
        self.client = AgentStateClient(
            base_url=base_url or os.getenv('AGENTSTATE_URL', 'http://localhost:8080'),
            namespace=namespace,
            api_key=api_key or os.getenv('AGENTSTATE_API_KEY')
        )
        self.namespace = namespace
    
    def save_agent_state(self, agent_id: str, role: str, state_data: Dict[str, Any], 
                        tags: Dict[str, str] = None) -> Dict[str, Any]:
        """Save agent state to AgentState"""
        agent_tags = {
            "framework": "crewai",
            "role": role,
            "namespace": self.namespace,
            **(tags or {})
        }
        
        return self.client.create_agent(
            agent_type="crewai-agent",
            body={
                "role": role,
                "state": state_data,
                "last_updated": time.time(),
                **state_data
            },
            tags=agent_tags,
            agent_id=agent_id
        )
    
    def get_agent_state(self, agent_id: str) -> Dict[str, Any]:
        """Get agent state from AgentState"""
        try:
            agent = self.client.get_agent(agent_id)
            return agent['body'].get('state', {})
        except:
            return {}
    
    def save_task_progress(self, task_id: str, task_data: Dict[str, Any]) -> Dict[str, Any]:
        """Save task progress to AgentState"""
        return self.client.create_agent(
            agent_type="crewai-task",
            body={
                "task_data": task_data,
                "timestamp": time.time()
            },
            tags={
                "framework": "crewai",
                "type": "task",
                "status": task_data.get('status', 'unknown')
            },
            agent_id=task_id
        )
    
    def get_crew_agents(self) -> List[Dict[str, Any]]:
        """Get all CrewAI agents from AgentState"""
        return self.client.query_agents({"framework": "crewai", "type": "crewai-agent"})
    
    def cleanup_demo_data(self):
        """Clean up demo agents and tasks"""
        agents = self.client.query_agents({"framework": "crewai"})
        for agent in agents:
            self.client.delete_agent(agent['id'])
        return len(agents)


# Global state manager instance
state_manager = AgentStateManager()


@tool
def save_research_findings(findings: str) -> str:
    """Save research findings to AgentState for sharing with other agents"""
    try:
        finding_data = {
            "content": findings,
            "timestamp": time.time(),
            "type": "research_finding"
        }
        
        result = state_manager.client.create_agent(
            agent_type="research-finding",
            body=finding_data,
            tags={
                "framework": "crewai",
                "type": "finding",
                "category": "research"
            }
        )
        
        return f"Research findings saved to AgentState with ID: {result['id']}"
    except Exception as e:
        return f"Error saving research findings: {str(e)}"


@tool
def get_shared_findings() -> str:
    """Get shared research findings from AgentState"""
    try:
        findings = state_manager.client.query_agents({
            "framework": "crewai",
            "type": "finding",
            "category": "research"
        })
        
        if not findings:
            return "No shared research findings available."
        
        result = "Shared Research Findings:\n"
        for finding in findings[-3:]:  # Get last 3 findings
            content = finding['body'].get('content', 'No content')
            timestamp = finding['body'].get('timestamp', 0)
            result += f"- {content} (saved at {time.ctime(timestamp)})\n"
        
        return result
    except Exception as e:
        return f"Error retrieving shared findings: {str(e)}"


@tool
def coordinate_with_agents(message: str) -> str:
    """Send coordination message to all agents in the crew"""
    try:
        coord_msg = state_manager.client.create_agent(
            agent_type="coordination-message",
            body={
                "message": message,
                "timestamp": time.time(),
                "sender": "crew-coordinator"
            },
            tags={
                "framework": "crewai",
                "type": "coordination"
            }
        )
        
        # Get count of active agents
        agents = state_manager.get_crew_agents()
        
        return f"Coordination message sent to {len(agents)} agents. Message ID: {coord_msg['id']}"
    except Exception as e:
        return f"Error sending coordination message: {str(e)}"


class CrewAIAgentStateDemo:
    """Demo class for CrewAI + AgentState integration"""
    
    def __init__(self):
        self.llm = ChatOpenAI(
            model="gpt-3.5-turbo",
            temperature=0.7
        )
        self.state_manager = state_manager
        
    def create_research_crew(self):
        """Create a research crew with persistent state"""
        
        # Research Agent
        researcher = Agent(
            role='Senior Research Analyst',
            goal='Conduct thorough research and analysis on given topics',
            backstory="""You are an experienced research analyst with a keen eye for detail.
            You excel at gathering information, analyzing data, and providing comprehensive insights.
            You always save important findings for your team to use.""",
            verbose=True,
            allow_delegation=False,
            tools=[save_research_findings, get_shared_findings],
            llm=self.llm
        )
        
        # Writer Agent  
        writer = Agent(
            role='Technical Writer',
            goal='Create engaging and informative content based on research findings',
            backstory="""You are a skilled technical writer who transforms complex research
            into clear, engaging content. You work closely with researchers and always
            check for shared findings before starting your work.""",
            verbose=True,
            allow_delegation=False,
            tools=[get_shared_findings, coordinate_with_agents],
            llm=self.llm
        )
        
        # Quality Assurance Agent
        qa_agent = Agent(
            role='Quality Assurance Specialist', 
            goal='Review and ensure the quality of all deliverables',
            backstory="""You are a meticulous QA specialist who ensures all work meets
            high standards. You coordinate with the team to gather feedback and
            maintain quality throughout the process.""",
            verbose=True,
            allow_delegation=False,
            tools=[get_shared_findings, coordinate_with_agents],
            llm=self.llm
        )
        
        return [researcher, writer, qa_agent]
    
    def create_research_tasks(self, agents):
        """Create tasks for the research crew"""
        researcher, writer, qa_agent = agents
        
        # Research Task
        research_task = Task(
            description="""Research the latest trends in AI agent frameworks and tools.
            Focus on:
            1. Popular frameworks like LangChain, CrewAI, AutoGen
            2. State management solutions for AI agents
            3. Best practices for multi-agent systems
            
            Save your key findings using the available tools so other team members can access them.""",
            agent=researcher,
            expected_output="Comprehensive research report with key findings saved to shared state"
        )
        
        # Writing Task
        writing_task = Task(
            description="""Based on the research findings, write a comprehensive article about 
            AI agent frameworks and state management.
            
            First check for shared research findings, then create an engaging article that covers:
            1. Overview of current AI agent landscape
            2. Comparison of different frameworks
            3. Importance of state management in multi-agent systems
            4. Future trends and recommendations
            
            Coordinate with the team if you need additional information.""",
            agent=writer,
            expected_output="Well-written article about AI agent frameworks (1500-2000 words)"
        )
        
        # QA Task
        qa_task = Task(
            description="""Review the research and writing work completed by the team.
            
            Check shared findings and coordinate with team members to ensure:
            1. Research is accurate and comprehensive
            2. Article is well-structured and engaging
            3. All key points from research are covered
            4. Content meets professional standards
            
            Provide feedback and suggestions for improvement.""",
            agent=qa_agent,
            expected_output="Quality assessment report with recommendations"
        )
        
        return [research_task, writing_task, qa_task]
    
    def run_crew_demo(self):
        """Run the CrewAI demo with AgentState integration"""
        print("🤖 CrewAI + AgentState Integration Demo")
        print("=" * 50)
        
        # Create crew
        agents = self.create_research_crew()
        tasks = self.create_research_tasks(agents)
        
        # Save agent states before starting
        for i, agent in enumerate(agents):
            agent_id = f"crewai-agent-{i}"
            self.state_manager.save_agent_state(
                agent_id=agent_id,
                role=agent.role,
                state_data={
                    "status": "initialized",
                    "goal": agent.goal,
                    "backstory": agent.backstory
                },
                tags={"position": str(i)}
            )
        
        # Create and run crew
        crew = Crew(
            agents=agents,
            tasks=tasks,
            process=Process.sequential,
            verbose=2
        )
        
        print("\n🚀 Starting crew execution...")
        result = crew.kickoff()
        
        print(f"\n✅ Crew execution completed!")
        print(f"Result: {result}")
        
        # Save final results
        self.state_manager.save_task_progress(
            "final-crew-result",
            {
                "result": str(result),
                "status": "completed",
                "completion_time": time.time()
            }
        )
        
        # Display AgentState data
        print("\n📊 AgentState Data Summary:")
        agents_data = self.state_manager.get_crew_agents() 
        findings = self.state_manager.client.query_agents({"framework": "crewai", "type": "finding"})
        coord_messages = self.state_manager.client.query_agents({"framework": "crewai", "type": "coordination"})
        
        print(f"  Agents tracked: {len(agents_data)}")
        print(f"  Research findings: {len(findings)}")
        print(f"  Coordination messages: {len(coord_messages)}")
        
        return result
    
    def cleanup(self):
        """Clean up demo data"""
        count = self.state_manager.cleanup_demo_data()
        print(f"🧹 Cleaned up {count} demo objects from AgentState")


def main():
    """Run the CrewAI + AgentState demo"""
    if not os.getenv('OPENAI_API_KEY'):
        print("❌ Please set OPENAI_API_KEY environment variable")
        return
    
    demo = CrewAIAgentStateDemo()
    
    try:
        demo.run_crew_demo()
    except KeyboardInterrupt:
        print("\n⏹️  Demo interrupted by user")
    except Exception as e:
        print(f"❌ Demo failed: {e}")
    finally:
        demo.cleanup()


if __name__ == "__main__":
    main()