# 🤖 AgentState TypeScript SDK

**Firebase for AI Agents** - TypeScript/JavaScript SDK for persistent agent state management.

[![npm version](https://img.shields.io/npm/v/agentstate.svg)](https://www.npmjs.com/package/agentstate)
[![License](https://img.shields.io/npm/l/agentstate.svg)](https://github.com/ayushmi/agentstate/blob/main/LICENSE)

## 🚀 Quick Start

### Installation

```bash
npm install agentstate
```

### Basic Usage

```typescript
import { AgentStateClient } from 'agentstate';

// Connect to AgentState server
const client = new AgentStateClient('http://localhost:8080', 'my-app');

// Create an agent
const agent = await client.createAgent('chatbot', {
  name: 'CustomerBot',
  status: 'active',
  conversations: 0
}, {
  team: 'support',
  environment: 'production'
});

console.log(`Created agent: ${agent.id}`);

// Update agent state
const updated = await client.createAgent('chatbot', {
  name: 'CustomerBot', 
  status: 'busy',
  conversations: 5
}, {
  team: 'support',
  environment: 'production'
}, agent.id); // Update existing agent

// Query agents
const supportAgents = await client.queryAgents({ team: 'support' });
console.log(`Found ${supportAgents.length} support agents`);

// Get specific agent
const retrieved = await client.getAgent(agent.id);
console.log(`Agent status: ${retrieved.body.status}`);
```

### JavaScript (Node.js) Usage

```javascript
const { AgentStateClient } = require('agentstate');

const client = new AgentStateClient('http://localhost:8080', 'my-app');

async function main() {
  // Create workflow agent
  const workflow = await client.createAgent('workflow', {
    name: 'DataProcessor',
    status: 'idle',
    queueSize: 0
  }, {
    capability: 'data-processing'
  });

  console.log(`Created workflow: ${workflow.id}`);
  
  // Update with progress
  await client.createAgent('workflow', {
    name: 'DataProcessor',
    status: 'processing',
    queueSize: 5,
    currentJob: 'user-analytics'
  }, {
    capability: 'data-processing'
  }, workflow.id);
  
  console.log('Workflow updated!');
}

main().catch(console.error);
```

## 📚 API Reference

### AgentStateClient

#### `constructor(baseUrl?, namespace?)`

Initialize the client.

- `baseUrl`: AgentState server URL (default: "http://localhost:8080")
- `namespace`: Namespace for organizing agents (default: "default")

#### `createAgent(agentType, body, tags?, agentId?): Promise<Agent>`

Create or update an agent.

- `agentType`: Agent category (e.g., "chatbot", "workflow", "classifier")
- `body`: Agent state data (object)
- `tags`: Key-value pairs for querying (optional)
- `agentId`: Specific ID for updates (optional)

#### `getAgent(agentId): Promise<Agent>`

Get agent by ID.

#### `queryAgents(tags?): Promise<Agent[]>`

Query agents by tags.

#### `deleteAgent(agentId): Promise<void>`

Delete an agent.

#### `healthCheck(): Promise<boolean>`

Check server health.

## 🎯 Usage Examples

### Multi-Agent Coordination

```typescript
import { AgentStateClient } from 'agentstate';

const client = new AgentStateClient('http://localhost:8080', 'multi-agent');

// Create coordinator
const coordinator = await client.createAgent('coordinator', {
  status: 'active',
  workers: [],
  tasksQueued: 50
}, { role: 'coordinator' });

// Create workers
const workers = [];
for (let i = 0; i < 3; i++) {
  const worker = await client.createAgent('worker', {
    status: 'idle',
    processedToday: 0,
    coordinatorId: coordinator.id
  }, { 
    role: 'worker',
    coordinator: coordinator.id
  });
  workers.push(worker);
}

console.log('Multi-agent system initialized!');
```

### Real-time State Updates

```typescript
// Simulate processing pipeline
const processor = await client.createAgent('processor', {
  status: 'idle',
  currentTask: null,
  processedCount: 0
}, { type: 'processor' });

const tasks = ['task_1', 'task_2', 'task_3'];

for (const [i, task] of tasks.entries()) {
  // Update to processing
  await client.createAgent('processor', {
    status: 'processing',
    currentTask: task,
    processedCount: i
  }, { type: 'processor' }, processor.id);
  
  // Simulate work
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  // Update to completed
  await client.createAgent('processor', {
    status: 'idle',
    currentTask: null,
    processedCount: i + 1
  }, { type: 'processor' }, processor.id);
}
```

### Error Handling

```typescript
try {
  const agent = await client.createAgent('test', { data: 'example' });
  console.log(`Created: ${agent.id}`);
} catch (error) {
  if (error.response?.status === 404) {
    console.error('AgentState server not found');
  } else if (error.response?.status >= 500) {
    console.error('Server error:', error.message);
  } else {
    console.error('Request failed:', error.message);
  }
}
```

## 🔧 TypeScript Support

Full TypeScript support with comprehensive type definitions:

```typescript
import { AgentStateClient, Agent, Tags } from 'agentstate';

interface MyAgentBody {
  name: string;
  status: 'active' | 'idle' | 'busy';
  metrics: {
    requests: number;
    uptime: number;
  };
}

const client = new AgentStateClient();

const agent: Agent = await client.createAgent('my-type', {
  name: 'TypedAgent',
  status: 'active',
  metrics: { requests: 0, uptime: 0 }
} as MyAgentBody, {
  environment: 'production'
});
```

## 🔗 Links

- **GitHub**: https://github.com/ayushmi/agentstate
- **Documentation**: https://github.com/ayushmi/agentstate#readme
- **Issues**: https://github.com/ayushmi/agentstate/issues
- **npm**: https://www.npmjs.com/package/agentstate

---

**Ready to build AI agents with persistent state!** 🚀

