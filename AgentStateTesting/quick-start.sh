#!/bin/bash
# AgentState Testing Quick Start Script
# =====================================
# Quickly set up and run AgentState testing environments

set -e

echo "🚀 AgentState Testing Quick Start"
echo "================================="

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo "📋 Checking prerequisites..."

if ! command_exists docker; then
    echo "❌ Docker is required but not installed."
    exit 1
fi

if ! command_exists python3; then
    echo "❌ Python 3 is required but not installed."
    exit 1
fi

if ! command_exists node; then
    echo "❌ Node.js is required but not installed."
    exit 1
fi

echo "✅ All prerequisites found"

# Start AgentState server
echo ""
echo "🐳 Starting AgentState server with Docker..."
docker-compose up -d agentstate

echo "⏳ Waiting for AgentState server to be ready..."
sleep 5

# Health check
for i in {1..30}; do
    if curl -s http://localhost:8080/health > /dev/null; then
        echo "✅ AgentState server is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ AgentState server failed to start"
        exit 1
    fi
    sleep 1
done

# Set up Python environment
echo ""
echo "🐍 Setting up Python testing environment..."
cd python-tests

if [ ! -d "venv" ]; then
    python3 -m venv venv
fi

source venv/bin/activate
pip install -r requirements.txt

echo "🧪 Running Python SDK tests..."
python basic-sdk-test.py

cd ..

# Set up Node.js environment  
echo ""
echo "📦 Setting up Node.js testing environment..."
cd nodejs-tests

npm install

echo "🧪 Running Node.js SDK tests..."
npm test

cd ..

echo ""
echo "🎉 Quick start completed successfully!"
echo ""
echo "Next steps:"
echo "1. Copy .env.example to .env and configure API keys"
echo "2. Run specific framework examples:"
echo "   - Python LangChain: cd python-tests && python langchain-example/langchain_agentstate_demo.py"
echo "   - Python CrewAI: cd python-tests && python crewai-example/crewai_agentstate_demo.py"
echo "   - Node.js LangChain: cd nodejs-tests/langchainjs-example && npm start"
echo "3. Stop the server: docker-compose down"
echo ""
echo "📚 See README.md for more detailed information"