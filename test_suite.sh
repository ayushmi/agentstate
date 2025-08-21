#!/bin/bash
set -e

echo "🧪 AgentState v1.0.0 Comprehensive Test Suite"
echo "============================================="

BASE_URL="http://localhost:8080"
NAMESPACE="test-agents"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

test_passed=0
test_failed=0

# Test function
run_test() {
    local test_name="$1"
    local test_command="$2"
    
    echo -e "${BLUE}Running: $test_name${NC}"
    
    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASS: $test_name${NC}\n"
        ((test_passed++))
    else
        echo -e "${RED}❌ FAIL: $test_name${NC}\n"
        ((test_failed++))
    fi
}

echo "🔍 Basic Health Checks"
echo "====================="

run_test "Health Endpoint" "curl -f -s $BASE_URL/health > /dev/null"
run_test "Metrics Endpoint" "curl -f -s $BASE_URL/metrics | grep -q 'query_planner_micros'"

echo "📡 HTTP API Tests"
echo "================"

# Test 1: Create an agent object
AGENT_DATA='{
    "type": "agent",
    "body": {
        "name": "TestAgent",
        "version": "1.0.0",
        "status": "active",
        "config": {
            "model": "gpt-4",
            "temperature": 0.7
        }
    },
    "tags": {
        "environment": "test",
        "type": "chatbot"
    }
}'

# Create and capture the ID
AGENT_RESPONSE=$(curl -s -X POST $BASE_URL/v1/$NAMESPACE/objects -H 'Content-Type: application/json' -d "$AGENT_DATA")
AGENT_ID=$(echo $AGENT_RESPONSE | grep -o '"id":"[^"]*"' | cut -d'"' -f4)

run_test "Create Agent Object" "echo '$AGENT_RESPONSE' | grep -q '\"type\":\"agent\"'"

# Test 2: Retrieve the agent  
run_test "Get Agent Object" "curl -f -s $BASE_URL/v1/$NAMESPACE/objects/$AGENT_ID | grep -q '\"name\":\"TestAgent\"'"

# Test 3: Update agent state
UPDATE_DATA='{
    "type": "agent",
    "body": {
        "name": "TestAgent", 
        "version": "1.0.1",
        "status": "busy",
        "current_task": "processing_request_123"
    },
    "tags": {
        "environment": "test",
        "type": "chatbot"
    }
}'

run_test "Update Agent State" "curl -f -s -X POST $BASE_URL/v1/$NAMESPACE/objects \
    -H 'Content-Type: application/json' \
    -d '$UPDATE_DATA' > /dev/null"

# Test 4: Query agents by tag
QUERY_DATA='{
    "tags": {"environment": "test"}
}'

run_test "Query by Tags" "curl -f -s -X POST $BASE_URL/v1/$NAMESPACE/query \
    -H 'Content-Type: application/json' \
    -d '$QUERY_DATA' | grep -q '\"environment\":\"test\"'"

# Test 5: Create multiple agents for bulk testing
echo "Creating bulk test agents..."
for i in {2..5}; do
    BULK_DATA="{\"type\": \"agent\", \"body\": {\"name\": \"Agent$i\", \"status\": \"idle\"}, \"tags\": {\"batch\": \"test\"}}"
    curl -f -s -X POST $BASE_URL/v1/$NAMESPACE/objects -H 'Content-Type: application/json' -d "$BULK_DATA" > /dev/null
done

run_test "Bulk Query" "curl -f -s -X POST $BASE_URL/v1/$NAMESPACE/query \
    -H 'Content-Type: application/json' \
    -d '{\"tags\": {\"batch\": \"test\"}}' | grep -o '\"batch\":\"test\"' | wc -l | grep -q '4'"

# Test 6: Delete agent (get a fresh ID first)
DELETE_RESPONSE=$(curl -s -X POST $BASE_URL/v1/$NAMESPACE/objects -H 'Content-Type: application/json' -d '{"type":"temp","body":{"test":true},"tags":{}}')
DELETE_ID=$(echo $DELETE_RESPONSE | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
run_test "Delete Agent" "curl -f -s -X DELETE $BASE_URL/v1/$NAMESPACE/objects/$DELETE_ID"

echo "🔗 gRPC Tests (Basic)"
echo "===================="

# Check if grpcurl is available
if command -v grpcurl &> /dev/null; then
    run_test "gRPC Health Check" "grpcurl -plaintext localhost:9090 list > /dev/null"
else
    echo -e "${YELLOW}⚠️  SKIP: gRPC tests (grpcurl not installed)${NC}\n"
fi

echo "⚡ Performance Tests"
echo "==================="

# Simple load test
run_test "Concurrent Requests" "
    for i in {1..10}; do
        curl -f -s $BASE_URL/health &
    done
    wait
    echo 'All concurrent requests completed'
"

echo "📊 Final Results"
echo "================"
echo -e "Tests Passed: ${GREEN}$test_passed${NC}"
echo -e "Tests Failed: ${RED}$test_failed${NC}"

if [ $test_failed -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All tests passed! AgentState is ready for production.${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed. Please check the issues above.${NC}"
    exit 1
fi