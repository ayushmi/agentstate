#!/bin/bash
set -e

echo "🚀 Publishing AgentState SDKs v1.0.0"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}📋 Pre-publication checks...${NC}"

# Check if we're in the right directory
if [ ! -f "README.md" ] || [ ! -d "sdk-py" ] || [ ! -d "sdk-ts" ]; then
    echo -e "${RED}❌ Error: Run this script from the AgentState project root${NC}"
    exit 1
fi

# Check if AgentState server is running
echo "🔍 Checking AgentState server..."
if ! curl -f -s http://localhost:8080/health > /dev/null; then
    echo -e "${RED}❌ Error: AgentState server is not running on localhost:8080${NC}"
    echo "Start it with: docker run -p 8080:8080 -p 9090:9090 agentstate:latest"
    exit 1
fi

echo -e "${GREEN}✅ Server is running${NC}"

# Test both SDKs
echo -e "${BLUE}🧪 Testing SDKs before publication...${NC}"

echo "Testing Python SDK..."
if ! python test_python_sdk.py; then
    echo -e "${RED}❌ Python SDK tests failed${NC}"
    exit 1
fi

echo "Testing TypeScript SDK..."
if ! node test_typescript_sdk.js; then
    echo -e "${RED}❌ TypeScript SDK tests failed${NC}" 
    exit 1
fi

echo -e "${GREEN}✅ All SDK tests passed${NC}"

# Build TypeScript SDK
echo -e "${BLUE}🔨 Building TypeScript SDK...${NC}"
cd sdk-ts
npm run build
cd ..

# Python SDK - Create distribution packages
echo -e "${BLUE}📦 Building Python SDK distribution...${NC}"
cd sdk-py

# Clean previous builds
rm -rf build/ dist/ *.egg-info/

# Build source and wheel distributions
python setup.py sdist bdist_wheel

echo -e "${GREEN}✅ Python SDK built successfully${NC}"
echo "   📁 Distribution files created in sdk-py/dist/"
ls -la dist/

cd ..

# TypeScript SDK - Prepare for npm
echo -e "${BLUE}📦 Preparing TypeScript SDK for npm...${NC}"
cd sdk-ts

# Ensure we have the built files
if [ ! -f "dist/index.js" ] || [ ! -f "dist/index.d.ts" ]; then
    echo -e "${RED}❌ TypeScript build files missing${NC}"
    exit 1
fi

echo -e "${GREEN}✅ TypeScript SDK ready for npm${NC}"
echo "   📁 Built files in sdk-ts/dist/"
ls -la dist/

cd ..

echo -e "${GREEN}🎉 SDKs ready for publication!${NC}"
echo ""
echo -e "${YELLOW}📋 Next steps:${NC}"
echo ""
echo -e "${BLUE}For Python SDK (PyPI):${NC}"
echo "  cd sdk-py"
echo "  # Test upload to TestPyPI first:"
echo "  twine upload --repository testpypi dist/*"
echo "  # Then upload to PyPI:"
echo "  twine upload dist/*"
echo ""
echo -e "${BLUE}For TypeScript SDK (npm):${NC}"
echo "  cd sdk-ts"  
echo "  # Login to npm (if not already):"
echo "  npm login"
echo "  # Publish to npm:"
echo "  npm publish"
echo ""
echo -e "${YELLOW}⚠️  Remember to:${NC}"
echo "  - Ensure you have accounts on PyPI and npm"
echo "  - Have proper authentication set up"
echo "  - Test on TestPyPI first before publishing to production PyPI"
echo "  - Update version numbers if republishing"
echo ""
echo -e "${GREEN}🚀 Ready to make AgentState available to the world!${NC}"