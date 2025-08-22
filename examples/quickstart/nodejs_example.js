#!/usr/bin/env node
/**
 * 🤖 AgentState Node.js SDK Example
 * =================================
 * 
 * This example demonstrates how to use AgentState as "Firebase for AI Agents"
 * - Store agent state persistently 
 * - Query agents by tags
 * - Subscribe to real-time updates  
 * - Manage agent lifecycle
 * 
 * Install: npm install agentstate
 * Usage: node nodejs_example.js
 */

const { AgentStateClient } = require('agentstate');

async function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
    console.log('🤖 AgentState Node.js Example');
    console.log('=============================');
    
    // Initialize client
    const client = new AgentStateClient('http://localhost:8080', 'my-ai-app');
    
    try {
        // 1. Create a conversational AI agent
        console.log('\n💬 Creating conversational AI agent...');
        const conversationAgent = await client.createAgent(
            'conversational-ai',
            {
                name: 'SlackBot',
                platform: 'slack',
                status: 'online',
                activeChannels: ['#general', '#support'],
                personality: {
                    tone: 'friendly',
                    expertise: ['customer-support', 'general-qa']
                },
                metrics: {
                    messagesHandled: 0,
                    usersSatisfied: 0,
                    avgResponseTime: 0
                }
            },
            {
                environment: 'production',
                platform: 'slack',
                team: 'customer-success',
                version: '2.1.0'
            }
        );
        const botId = conversationAgent.id;
        console.log(`✅ Created conversational AI: ${botId}`);

        // 2. Create a workflow automation agent
        console.log('\n⚙️ Creating workflow automation agent...');
        const workflowAgent = await client.createAgent(
            'workflow-automation',
            {
                name: 'CRM-SyncAgent',
                status: 'active',
                currentWorkflow: null,
                scheduledTasks: [],
                integrations: ['salesforce', 'hubspot', 'slack'],
                successRate: 0.98
            },
            {
                environment: 'production',
                team: 'sales-ops',
                capability: 'crm-sync'
            }
        );
        const workflowId = workflowAgent.id;
        console.log(`✅ Created workflow agent: ${workflowId}`);

        // 3. Simulate agent activity (bot handles messages)
        console.log('\n📨 Simulating bot activity...');
        const messages = [
            { user: 'john', text: 'How do I reset my password?', channel: '#support' },
            { user: 'jane', text: 'What are your business hours?', channel: '#general' },
            { user: 'bob', text: 'I need help with billing', channel: '#support' }
        ];

        for (let i = 0; i < messages.length; i++) {
            const msg = messages[i];
            
            // Update bot state to show it's processing
            await client.createAgent(
                'conversational-ai',
                {
                    name: 'SlackBot',
                    platform: 'slack', 
                    status: 'processing',
                    activeChannels: ['#general', '#support'],
                    currentMessage: {
                        user: msg.user,
                        channel: msg.channel,
                        text: msg.text,
                        timestamp: new Date().toISOString()
                    },
                    personality: {
                        tone: 'friendly',
                        expertise: ['customer-support', 'general-qa']
                    },
                    metrics: {
                        messagesHandled: i + 1,
                        usersSatisfied: i,
                        avgResponseTime: 1.5
                    }
                },
                {
                    environment: 'production',
                    platform: 'slack',
                    team: 'customer-success',
                    version: '2.1.0'
                },
                botId
            );
            
            console.log(`  Processing: "${msg.text}" from ${msg.user} in ${msg.channel}`);
            await sleep(1500); // Simulate processing time
        }

        // Update to completed state
        await client.createAgent(
            'conversational-ai',
            {
                name: 'SlackBot',
                platform: 'slack',
                status: 'online',
                activeChannels: ['#general', '#support'],
                currentMessage: null,
                personality: {
                    tone: 'friendly', 
                    expertise: ['customer-support', 'general-qa']
                },
                metrics: {
                    messagesHandled: messages.length,
                    usersSatisfied: messages.length - 1, // One escalated
                    avgResponseTime: 1.5
                }
            },
            {
                environment: 'production',
                platform: 'slack',
                team: 'customer-success',
                version: '2.1.0'
            },
            botId
        );
        console.log('✅ Message processing complete');

        // 4. Query agents by team
        console.log('\n🔍 Querying agents by team...');
        const customerSuccessAgents = await client.queryAgents({ team: 'customer-success' });
        const salesOpsAgents = await client.queryAgents({ team: 'sales-ops' });
        
        console.log(`✅ Customer Success team: ${customerSuccessAgents.length} agents`);
        console.log(`✅ Sales Ops team: ${salesOpsAgents.length} agents`);

        // 5. Get detailed agent states
        console.log('\n📊 Agent Status Report:');
        const allAgents = await client.queryAgents({ environment: 'production' });
        
        for (const agent of allAgents) {
            const status = agent.body.status;
            const name = agent.body.name;
            const type = agent.type;
            console.log(`  📍 ${name} (${type}): ${status}`);
            
            if (agent.body.metrics) {
                const metrics = agent.body.metrics;
                console.log(`     📈 Messages: ${metrics.messagesHandled}, Satisfaction: ${metrics.usersSatisfied}`);
            }
        }

        // 6. Clean up
        console.log('\n🧹 Cleaning up test agents...');
        await client.deleteAgent(botId);
        await client.deleteAgent(workflowId);
        console.log('✅ Cleanup complete');

        console.log('\n🎉 Example complete! AgentState provides:');
        console.log('   📦 Persistent agent state storage');
        console.log('   🏷️  Rich tagging and querying system');
        console.log('   ⚡ Real-time state updates');
        console.log('   🔄 Complete agent lifecycle management');
        console.log('\nReady for your production AI applications! 🚀');

    } catch (error) {
        console.error('❌ Error:', error.message);
        if (error.response) {
            console.error('Response:', error.response.status, error.response.data);
        }
        console.log('Make sure AgentState server is running on http://localhost:8080');
    }
}

// Check if agentstate is available
try {
    require('agentstate');
    main();
} catch (e) {
    console.error('❌ Please install agentstate: npm install agentstate');
    process.exit(1);
}