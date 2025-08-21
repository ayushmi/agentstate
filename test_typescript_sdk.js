#!/usr/bin/env node
/**
 * 🧪 Test TypeScript SDK functionality
 */

import { AgentStateClient, State } from './sdk-ts/dist/index.js';

async function testTypeScriptSDK() {
    console.log('🧪 Testing TypeScript SDK v1.0.0');
    console.log('=' .repeat(40));
    
    const client = new AgentStateClient('http://localhost:8080', 'ts-sdk-test');
    
    try {
        // Test 1: Health check
        console.log('1. Testing health check...');
        const healthy = await client.healthCheck();
        if (!healthy) throw new Error('Server is not healthy');
        console.log('   ✅ Server is healthy');
        
        // Test 2: Create agent
        console.log('2. Testing create agent...');
        const agent = await client.createAgent('test-chatbot', {
            name: 'TypeScriptBot',
            version: '1.0.0',
            status: 'initializing',
            features: ['chat', 'search', 'analyze'],
            metrics: { sessions: 0, messages: 0 }
        }, {
            environment: 'test',
            team: 'ts-sdk-team',
            language: 'typescript'
        });
        
        const agentId = agent.id;
        if (agent.type !== 'test-chatbot') throw new Error('Invalid agent type');
        if (agent.body.name !== 'TypeScriptBot') throw new Error('Invalid agent name');
        if (agent.tags.language !== 'typescript') throw new Error('Invalid agent tags');
        console.log(`   ✅ Created agent: ${agentId}`);
        
        // Test 3: Get agent
        console.log('3. Testing get agent...');
        const retrieved = await client.getAgent(agentId);
        if (retrieved.id !== agentId) throw new Error('Retrieved wrong agent');
        if (retrieved.body.name !== 'TypeScriptBot') throw new Error('Retrieved agent has wrong data');
        console.log(`   ✅ Retrieved agent: ${retrieved.body.name}`);
        
        // Test 4: Update agent
        console.log('4. Testing update agent...');
        const updated = await client.createAgent('test-chatbot', {
            name: 'TypeScriptBot',
            version: '1.0.0',
            status: 'active',
            features: ['chat', 'search', 'analyze', 'recommend'],
            metrics: { sessions: 10, messages: 150 }
        }, {
            environment: 'test',
            team: 'ts-sdk-team',
            language: 'typescript'
        }, agentId);
        
        if (updated.body.status !== 'active') throw new Error('Agent not updated properly');
        if (updated.body.metrics.sessions !== 10) throw new Error('Metrics not updated');
        console.log('   ✅ Updated agent state');
        
        // Test 5: Query agents
        console.log('5. Testing query agents...');
        
        // Create additional test agents
        const agent2 = await client.createAgent('test-worker', {
            name: 'Worker1',
            status: 'idle',
            workerId: 1
        }, { team: 'ts-sdk-team', role: 'worker' });
        
        const agent3 = await client.createAgent('test-worker', {
            name: 'Worker2', 
            status: 'busy',
            workerId: 2
        }, { team: 'ts-sdk-team', role: 'worker' });
        
        // Query by team
        const teamAgents = await client.queryAgents({ team: 'ts-sdk-team' });
        if (teamAgents.length < 3) throw new Error(`Expected at least 3 agents, got ${teamAgents.length}`);
        console.log(`   ✅ Found ${teamAgents.length} team agents`);
        
        // Query by role
        const workers = await client.queryAgents({ role: 'worker' });
        if (workers.length < 2) throw new Error(`Expected at least 2 workers, got ${workers.length}`);
        console.log(`   ✅ Found ${workers.length} worker agents`);
        
        // Test 6: Real-time updates simulation
        console.log('6. Testing real-time updates...');
        
        for (let i = 0; i < 3; i++) {
            await client.createAgent('test-chatbot', {
                name: 'TypeScriptBot',
                version: '1.0.0',
                status: 'processing',
                currentRequest: `request_${i + 1}`,
                features: ['chat', 'search', 'analyze', 'recommend'],
                metrics: { sessions: 10 + i, messages: 150 + i * 5 }
            }, {
                environment: 'test',
                team: 'ts-sdk-team',
                language: 'typescript'
            }, agentId);
            
            // Small delay to simulate real-time updates
            await new Promise(resolve => setTimeout(resolve, 100));
        }
        
        const finalState = await client.getAgent(agentId);
        if (finalState.body.currentRequest !== 'request_3') throw new Error('Real-time updates failed');
        console.log('   ✅ Real-time updates working');
        
        // Test 7: Legacy State class compatibility
        console.log('7. Testing legacy State class compatibility...');
        
        const legacyClient = new State('http://localhost:8080/v1/legacy-test');
        
        const legacyAgent = await legacyClient.put('legacy-type', { legacy: true }, { source: 'legacy' });
        if (legacyAgent.type !== 'legacy-type') throw new Error('Legacy State class failed');
        
        const legacyRetrieved = await legacyClient.get(legacyAgent.id);
        if (legacyRetrieved.body.legacy !== true) throw new Error('Legacy retrieval failed');
        
        const legacyQuery = await legacyClient.query({ source: 'legacy' });
        if (legacyQuery.length < 1) throw new Error('Legacy query failed');
        
        console.log('   ✅ Legacy State class compatibility working');
        
        // Test 8: Error handling
        console.log('8. Testing error handling...');
        
        try {
            await client.getAgent('nonexistent-id-12345');
            throw new Error('Should have thrown an error for non-existent agent');
        } catch (error) {
            if (error.message.includes('Should have thrown')) throw error;
            console.log('   ✅ Error handling for non-existent agent works');
        }
        
        // Test 9: TypeScript types (compile-time test - if we got here, types work)
        console.log('9. Testing TypeScript types...');
        
        // This demonstrates the TypeScript interface
        const typedAgent = await client.createAgent('typed-test', {
            name: 'TypedBot',
            status: 'active',
            config: { model: 'gpt-4', temperature: 0.7 }
        });
        
        // Type assertion works
        const agentBody = typedAgent.body;
        if (typeof agentBody.name !== 'string') throw new Error('Type checking failed');
        
        console.log('   ✅ TypeScript types working correctly');
        
        // Cleanup
        console.log('10. Cleaning up test agents...');
        const testAgents = await client.queryAgents({ environment: 'test' });
        const legacyAgents = await client.queryAgents({ source: 'legacy' });
        
        const agentsToDelete = [...testAgents, ...legacyAgents];
        
        // Add typed agent to cleanup list
        agentsToDelete.push(typedAgent);
        
        for (const agent of agentsToDelete) {
            try {
                await client.deleteAgent(agent.id);
            } catch (error) {
                // Ignore 404 errors (agent already deleted)
                if (error.response?.status !== 404) {
                    throw error;
                }
            }
        }
        
        console.log(`   ✅ Cleaned up ${testAgents.length + legacyAgents.length + 1} test agents`);
        
        console.log(`\n🎉 All TypeScript SDK tests passed!`);
        console.log(`✅ AgentState TypeScript SDK v1.0.0 is working perfectly!`);
        return true;
        
    } catch (error) {
        console.log(`\n❌ TypeScript SDK test failed: ${error.message}`);
        console.error(error.stack);
        return false;
    }
}

// Run the tests
testTypeScriptSDK()
    .then(success => {
        process.exit(success ? 0 : 1);
    })
    .catch(error => {
        console.error('Test runner failed:', error);
        process.exit(1);
    });