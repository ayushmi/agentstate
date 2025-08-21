# AgentState Testing & Examples

This directory contains testing environments and examples for AgentState - "Firebase for AI Agents".

## Structure

```
AgentStateTesting/
├── README.md                    # This file
├── docker-compose.yml           # Local AgentState server setup
├── python-tests/                # Python SDK testing & examples
│   ├── requirements.txt
│   ├── venv/                   # Python virtual environment
│   ├── basic-sdk-test.py       # Basic SDK functionality tests
│   ├── langchain-example/      # LangChain integration example
│   ├── crewai-example/         # CrewAI integration example
│   └── autogen-example/        # AutoGen integration example
├── nodejs-tests/               # Node.js/TypeScript SDK testing & examples
│   ├── package.json
│   ├── node_modules/
│   ├── basic-sdk-test.js       # Basic SDK functionality tests
│   ├── langchainjs-example/    # LangChain.js integration example
│   └── openai-agents-example/  # OpenAI assistants integration example
├── go-tests/                   # Go SDK testing (future)
├── rust-tests/                 # Rust SDK testing (future)
└── performance-tests/          # Load testing and benchmarks
    ├── load-test.py
    └── benchmark-results/
```

## Usage

### 1. Start AgentState Server

```bash
# Start AgentState server with Docker (pulls from Docker Hub)
docker-compose up -d

# Alternative: Run directly with Docker
docker run -p 8080:8080 ayushmi/agentstate:latest
```

### 2. Run Python Tests

```bash
cd python-tests
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
python basic-sdk-test.py
```

### 3. Run Node.js Tests

```bash
cd nodejs-tests
npm install
npm test
```

## Testing Scenarios

- **SDK Functionality**: Basic CRUD operations, authentication, error handling
- **Agentic Frameworks**: Integration with popular AI agent frameworks
- **Performance**: Load testing, concurrent operations, memory usage
- **Cloud Deployment**: Testing against hosted AgentState instances
- **Real-world Use Cases**: Multi-agent systems, workflow orchestration

## Environment Variables

```bash
# For testing against local server
export AGENTSTATE_URL=http://localhost:8080
export AGENTSTATE_API_KEY=your-test-api-key

# For testing against hosted cloud service
export AGENTSTATE_URL=https://api.agentstate.dev
export AGENTSTATE_API_KEY=your-production-api-key
```

This testing environment is completely isolated from the main AgentState codebase.