#!/usr/bin/env node
/**
 * Basic AgentState SDK Testing (Node.js/TypeScript)
 * ==================================================
 * 
 * Tests basic functionality of the AgentState TypeScript/JavaScript SDK including:
 * - Connection and health checks
 * - CRUD operations  
 * - Error handling
 * - Authentication
 */

import 'dotenv/config';
import { AgentStateClient } from 'agentstate';
import colors from 'colors';

// Configuration
const AGENTSTATE_URL = process.env.AGENTSTATE_URL || 'http://localhost:8080';
const AGENTSTATE_API_KEY = process.env.AGENTSTATE_API_KEY; // Optional
const NAMESPACE = 'testing';

// Helper functions for colored output
const success = (msg) => console.log(colors.green(`✅ ${msg}`));
const error = (msg) => console.log(colors.red(`❌ ${msg}`));
const info = (msg) => console.log(colors.blue(`ℹ️  ${msg}`));

class AgentStateSDKTester {
  constructor() {
    this.client = new AgentStateClient(AGENTSTATE_URL, NAMESPACE, AGENTSTATE_API_KEY);
    this.testAgents = []; // Track created agents for cleanup
  }

  async cleanup() {
    /**Clean up test agents*/
    for (const agentId of this.testAgents) {
      try {
        await this.client.deleteAgent(agentId);
        info(`Cleaned up agent: ${agentId}`);
      } catch (e) {
        error(`Failed to clean up agent ${agentId}: ${e.message}`);
      }
    }
    this.testAgents = [];
  }

  async testHealthCheck() {
    /**Test server health check*/
    info('Testing health check...');
    const isHealthy = await this.client.healthCheck();
    if (!isHealthy) {
      throw new Error('Server should be healthy');
    }
    success('Health check passed');
  }

  async testCreateAgent() {
    /**Test creating an agent*/
    info('Testing agent creation...');
    
    const agent = await this.client.createAgent(
      'test-bot',
      {
        name: 'TestBot',
        status: 'active',
        createdAt: Date.now()
      },
      {
        test: 'true',
        framework: 'sdk-test',
        environment: 'testing'
      }
    );
    
    if (agent.type !== 'test-bot') {
      throw new Error(`Expected type 'test-bot', got '${agent.type}'`);
    }
    if (agent.body.name !== 'TestBot') {
      throw new Error(`Expected name 'TestBot', got '${agent.body.name}'`);
    }
    if (agent.tags.test !== 'true') {
      throw new Error(`Expected tag test='true', got '${agent.tags.test}'`);
    }
    if (!agent.id || !agent.commit_seq || !agent.commit_ts) {
      throw new Error('Agent missing required fields: id, commit_seq, commit_ts');
    }
    
    this.testAgents.push(agent.id);
    success(`Created agent: ${agent.id}`);
    return agent;
  }

  async testGetAgent() {
    /**Test retrieving an agent by ID*/
    info('Testing agent retrieval...');
    
    // First create an agent
    const createdAgent = await this.testCreateAgent();
    
    // Then retrieve it
    const retrievedAgent = await this.client.getAgent(createdAgent.id);
    
    if (retrievedAgent.id !== createdAgent.id) {
      throw new Error('Retrieved agent ID does not match created agent ID');
    }
    if (JSON.stringify(retrievedAgent.body) !== JSON.stringify(createdAgent.body)) {
      throw new Error('Retrieved agent body does not match created agent body');
    }
    if (JSON.stringify(retrievedAgent.tags) !== JSON.stringify(createdAgent.tags)) {
      throw new Error('Retrieved agent tags do not match created agent tags');
    }
    
    success('Agent retrieval successful');
  }

  async testUpdateAgent() {
    /**Test updating an existing agent*/
    info('Testing agent update...');
    
    // Create initial agent
    const agent = await this.testCreateAgent();
    const originalSeq = agent.commit_seq;
    
    // Update the agent
    const updatedAgent = await this.client.createAgent(
      'test-bot',
      {
        name: 'UpdatedTestBot',
        status: 'busy',
        updatedAt: Date.now()
      },
      {
        test: 'true',
        framework: 'sdk-test',
        environment: 'testing',
        updated: 'true'
      },
      agent.id // Update existing
    );
    
    if (updatedAgent.id !== agent.id) {
      throw new Error('Updated agent ID does not match original agent ID');
    }
    if (updatedAgent.body.name !== 'UpdatedTestBot') {
      throw new Error(`Expected updated name 'UpdatedTestBot', got '${updatedAgent.body.name}'`);
    }
    if (updatedAgent.body.status !== 'busy') {
      throw new Error(`Expected updated status 'busy', got '${updatedAgent.body.status}'`);
    }
    if (updatedAgent.tags.updated !== 'true') {
      throw new Error(`Expected updated tag updated='true', got '${updatedAgent.tags.updated}'`);
    }
    if (updatedAgent.commit_seq <= originalSeq) {
      throw new Error('Updated agent should have higher commit_seq');
    }
    
    success('Agent update successful');
  }

  async testQueryAgents() {
    /**Test querying agents by tags*/
    info('Testing agent querying...');
    
    // Create multiple test agents
    const agents = [];
    for (let i = 0; i < 3; i++) {
      const agent = await this.client.createAgent(
        'query-test',
        {
          name: `QueryBot${i}`,
          index: i
        },
        {
          test: 'true',
          batch: 'query-test',
          priority: i % 2 === 0 ? 'high' : 'low'
        }
      );
      agents.push(agent);
      this.testAgents.push(agent.id);
    }
    
    // Query by batch tag
    const batchResults = await this.client.queryAgents({ batch: 'query-test' });
    if (batchResults.length !== 3) {
      throw new Error(`Expected 3 agents in batch, got ${batchResults.length}`);
    }
    success(`Found ${batchResults.length} agents in batch`);
    
    // Query by priority tag
    const highPriority = await this.client.queryAgents({
      batch: 'query-test',
      priority: 'high'
    });
    if (highPriority.length !== 2) { // indices 0 and 2
      throw new Error(`Expected 2 high priority agents, got ${highPriority.length}`);
    }
    success(`Found ${highPriority.length} high priority agents`);
  }

  async testDeleteAgent() {
    /**Test deleting an agent*/
    info('Testing agent deletion...');
    
    // Create an agent
    const agent = await this.testCreateAgent();
    const agentId = agent.id;
    
    // Verify it exists
    const retrieved = await this.client.getAgent(agentId);
    if (retrieved.id !== agentId) {
      throw new Error('Agent should exist before deletion');
    }
    
    // Delete it
    await this.client.deleteAgent(agentId);
    
    // Verify it's gone
    try {
      await this.client.getAgent(agentId);
      throw new Error('Agent should have been deleted');
    } catch (e) {
      if (!e.message.includes('404') && !e.message.toLowerCase().includes('not found')) {
        throw new Error(`Expected 404 error, got: ${e.message}`);
      }
    }
    
    success('Agent deletion successful');
    
    // Remove from cleanup list since it's already deleted
    const index = this.testAgents.indexOf(agentId);
    if (index > -1) {
      this.testAgents.splice(index, 1);
    }
  }

  async testErrorHandling() {
    /**Test error handling for invalid operations*/
    info('Testing error handling...');
    
    // Test getting non-existent agent
    try {
      await this.client.getAgent('non-existent-id');
      throw new Error('Should have raised an exception');
    } catch (e) {
      if (!e.message.includes('404') && !e.message.toLowerCase().includes('not found')) {
        throw new Error(`Expected 404 error, got: ${e.message}`);
      }
      success('404 error handled correctly');
    }
    
    // Test deleting non-existent agent
    try {
      await this.client.deleteAgent('non-existent-id');
      throw new Error('Should have raised an exception');
    } catch (e) {
      if (!e.message.includes('404') && !e.message.toLowerCase().includes('not found')) {
        throw new Error(`Expected 404 error, got: ${e.message}`);
      }
      success('Delete 404 error handled correctly');
    }
  }

  async runAllTests() {
    /**Run all tests*/
    console.log(colors.cyan('🧪 AgentState SDK Basic Testing'));
    console.log(`Server: ${AGENTSTATE_URL}`);
    console.log(`Namespace: ${NAMESPACE}`);
    console.log(`API Key: ${AGENTSTATE_API_KEY ? 'Set' : 'Not set'}`);
    console.log('-'.repeat(50));
    
    const tests = [
      this.testHealthCheck,
      this.testCreateAgent, 
      this.testGetAgent,
      this.testUpdateAgent,
      this.testQueryAgents,
      this.testDeleteAgent,
      this.testErrorHandling,
    ];
    
    let passed = 0;
    let failed = 0;
    
    for (const test of tests) {
      try {
        await test.call(this);
        await this.cleanup();
        passed++;
      } catch (e) {
        error(`Test ${test.name} failed: ${e.message}`);
        await this.cleanup();
        failed++;
      }
    }
    
    console.log('-'.repeat(50));
    console.log(colors.cyan(`Results: ${passed} passed, ${failed} failed`));
    
    if (failed === 0) {
      console.log(colors.green('🎉 All tests passed!'));
    } else {
      console.log(colors.red(`💥 ${failed} test(s) failed`));
      process.exit(1);
    }
  }
}

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  const tester = new AgentStateSDKTester();
  tester.runAllTests().catch(e => {
    error(`Test suite failed: ${e.message}`);
    process.exit(1);
  });
}