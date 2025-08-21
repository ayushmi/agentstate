#!/usr/bin/env node
/**
 * LangChain.js + AgentState Integration Example
 * ============================================= 
 * 
 * Demonstrates how to use AgentState as persistent storage for LangChain.js agents.
 * This example shows:
 * - Custom LangChain.js memory backed by AgentState
 * - Multi-agent conversation state management  
 * - Agent coordination through shared state
 */

import 'dotenv/config';
import { AgentStateClient } from 'agentstate';
import { ChatOpenAI } from '@langchain/openai';
import { DynamicTool } from '@langchain/core/tools';
import { ChatPromptTemplate, MessagesPlaceholder } from '@langchain/core/prompts';
import { createOpenAIFunctionsAgent, AgentExecutor } from 'langchain/agents';
import { BaseMessage, HumanMessage, AIMessage } from '@langchain/core/messages';
import { BaseChatMessageHistory } from '@langchain/core/chat_history';
import colors from 'colors';

/**
 * LangChain.js memory implementation using AgentState as backend
 */
class AgentStateMemory extends BaseChatMessageHistory {
  constructor(agentId, agentStateClient) {
    super();
    this.agentId = agentId;
    this.client = agentStateClient;
    this._ensureAgentExists();
  }

  async _ensureAgentExists() {
    try {
      await this.client.getAgent(this.agentId);
    } catch (error) {
      // Create agent if it doesn't exist
      await this.client.createAgent(
        'langchainjs-agent',
        {
          name: `LangChain.js Agent ${this.agentId}`,
          memory: { messages: [] },
          createdAt: Date.now()
        },
        {
          framework: 'langchainjs',
          type: 'chat-agent'
        },
        this.agentId
      );
    }
  }

  async getMessages() {
    const agent = await this.client.getAgent(this.agentId);
    const messagesData = agent.body.memory?.messages || [];
    
    return messagesData.map(msgData => {
      if (msgData.type === 'human') {
        return new HumanMessage(msgData.content);
      } else if (msgData.type === 'ai') {
        return new AIMessage(msgData.content);
      } else {
        return new AIMessage(msgData.content); // fallback
      }
    });
  }

  async addMessage(message) {
    // Get current agent state
    const agent = await this.client.getAgent(this.agentId);
    const currentMessages = agent.body.memory?.messages || [];
    
    // Add new message
    let msgData;
    if (message instanceof HumanMessage) {
      msgData = { type: 'human', content: message.content };
    } else if (message instanceof AIMessage) {
      msgData = { type: 'ai', content: message.content };
    } else {
      msgData = { type: 'system', content: message.content };
    }
    
    currentMessages.push(msgData);
    
    // Update agent in AgentState
    const updatedBody = {
      ...agent.body,
      memory: { messages: currentMessages },
      lastMessageAt: Date.now()
    };
    
    await this.client.createAgent(
      'langchainjs-agent',
      updatedBody,
      agent.tags,
      this.agentId
    );
  }

  async clear() {
    const agent = await this.client.getAgent(this.agentId);
    const updatedBody = {
      ...agent.body,
      memory: { messages: [] },
      clearedAt: Date.now()
    };
    
    await this.client.createAgent(
      'langchainjs-agent',
      updatedBody,
      agent.tags,
      this.agentId
    );
  }
}

/**
 * Create a calculator tool
 */
function createCalculatorTool() {
  return new DynamicTool({
    name: 'Calculator',
    description: 'Useful for mathematical calculations. Input should be a mathematical expression.',
    func: async (expression) => {
      try {
        // Basic safety check
        const allowedChars = /^[0-9+\-*/(). ]+$/;
        if (!allowedChars.test(expression)) {
          return 'Error: Invalid characters in expression';
        }
        
        const result = eval(expression);
        return `Result: ${result}`;
      } catch (error) {
        return `Error: ${error.message}`;
      }
    }
  });
}

/**
 * Create an agent coordination tool
 */
function createAgentCoordinationTool(agentStateClient) {
  return new DynamicTool({
    name: 'AgentCoordination',
    description: 'Send messages to coordinate with other agents in the system',
    func: async (message) => {
      try {
        // Get all langchainjs agents
        const agents = await agentStateClient.queryAgents({ framework: 'langchainjs' });
        
        // Store coordination message
        const coordMsg = await agentStateClient.createAgent(
          'coordination-message',
          {
            message: message,
            timestamp: Date.now(),
            sender: 'coordination-tool'
          },
          {
            type: 'coordination',
            framework: 'langchainjs'
          }
        );
        
        return `Coordination message sent to ${agents.length} agents. Message ID: ${coordMsg.id}`;
      } catch (error) {
        return `Error sending coordination message: ${error.message}`;
      }
    }
  });
}

/**
 * Demo class for LangChain.js + AgentState integration
 */
class LangChainJSAgentStateDemo {
  constructor() {
    // Initialize AgentState client
    this.agentState = new AgentStateClient(
      process.env.AGENTSTATE_URL || 'http://localhost:8080',
      'langchainjs-demo',
      process.env.AGENTSTATE_API_KEY
    );
    
    // Initialize OpenAI (requires OPENAI_API_KEY environment variable)
    this.llm = new ChatOpenAI({
      modelName: 'gpt-3.5-turbo',
      temperature: 0.7
    });
    
    // Create tools
    this.tools = [
      createCalculatorTool(),
      createAgentCoordinationTool(this.agentState)
    ];
    
    this.agents = new Map();
  }

  async createAgent(agentId, systemPrompt) {
    // Create AgentState-backed memory
    const memory = new AgentStateMemory(agentId, this.agentState);
    
    // Create prompt template
    const prompt = ChatPromptTemplate.fromMessages([
      ['system', systemPrompt],
      new MessagesPlaceholder('chat_history'),
      ['human', '{input}'],
      new MessagesPlaceholder('agent_scratchpad')
    ]);
    
    // Create agent
    const agent = await createOpenAIFunctionsAgent({
      llm: this.llm,
      tools: this.tools,
      prompt
    });
    
    const agentExecutor = new AgentExecutor({
      agent,
      tools: this.tools,
      verbose: true,
      memory
    });
    
    this.agents.set(agentId, agentExecutor);
    return agentExecutor;
  }

  async runMultiAgentDemo() {
    console.log(colors.cyan('🤖 LangChain.js + AgentState Multi-Agent Demo'));
    console.log(colors.cyan('='.repeat(50)));
    
    // Create two specialized agents
    const mathAgent = await this.createAgent(
      'math-specialist-js',
      'You are a mathematics specialist. Help with calculations and math problems. ' +
      'Use the Calculator tool for computations. You can also coordinate with other agents.'
    );
    
    const coordAgent = await this.createAgent(
      'coordinator-js',
      'You are a coordinator agent. Your job is to help organize tasks and coordinate ' +
      'between different agents. Use the AgentCoordination tool to communicate with other agents.'
    );
    
    console.log(colors.green('\n📊 Math Agent solving a problem:'));
    const mathResponse = await mathAgent.invoke({
      input: 'Calculate the volume of a sphere with radius 3, then tell other agents about this calculation'
    });
    console.log(colors.yellow(`Math Agent: ${mathResponse.output}`));
    
    console.log(colors.green('\n🎯 Coordinator organizing tasks:'));
    const coordResponse = await coordAgent.invoke({
      input: 'Check what coordination messages have been sent and organize a summary'
    });
    console.log(colors.yellow(`Coordinator: ${coordResponse.output}`));
    
    console.log(colors.green('\n📈 Checking AgentState for stored data:'));
    // Query all agents and coordination messages
    const agents = await this.agentState.queryAgents({ framework: 'langchainjs' });
    const coordMessages = await this.agentState.queryAgents({ type: 'coordination' });
    
    console.log(`Total agents in system: ${agents.length}`);
    console.log(`Coordination messages: ${coordMessages.length}`);
    
    for (const agent of agents) {
      if (agent.type === 'langchainjs-agent') {
        const messageCount = agent.body.memory?.messages?.length || 0;
        console.log(`  Agent ${agent.id}: ${messageCount} messages`);
      }
    }
  }

  async cleanup() {
    try {
      const agents = await this.agentState.queryAgents({ framework: 'langchainjs' });
      for (const agent of agents) {
        await this.agentState.deleteAgent(agent.id);
      }
      console.log(colors.green(`🧹 Cleaned up ${agents.length} test agents`));
    } catch (error) {
      console.log(colors.red(`❌ Cleanup error: ${error.message}`));
    }
  }
}

async function main() {
  if (!process.env.OPENAI_API_KEY) {
    console.log(colors.red('❌ Please set OPENAI_API_KEY environment variable'));
    return;
  }
  
  const demo = new LangChainJSAgentStateDemo();
  
  try {
    await demo.runMultiAgentDemo();
  } finally {
    await demo.cleanup();
  }
}

// Run the demo if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(error => {
    console.error(colors.red('Demo failed:', error.message));
    process.exit(1);
  });
}