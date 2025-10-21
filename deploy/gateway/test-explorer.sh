#!/bin/bash
set -e

echo "🧪 Testing IPPAN Blockchain Explorer"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test function
test_endpoint() {
    local url="$1"
    local description="$2"
    local expected_status="${3:-200}"
    
    echo -n "Testing $description... "
    
    if response=$(curl -s -w "%{http_code}" -o /dev/null "$url" 2>/dev/null); then
        if [ "$response" = "$expected_status" ]; then
            echo -e "${GREEN}✅ PASS${NC} (HTTP $response)"
            return 0
        else
            echo -e "${YELLOW}⚠️  WARN${NC} (HTTP $response, expected $expected_status)"
            return 1
        fi
    else
        echo -e "${RED}❌ FAIL${NC} (Connection failed)"
        return 1
    fi
}

# Test JSON endpoint
test_json_endpoint() {
    local url="$1"
    local description="$2"
    
    echo -n "Testing $description... "
    
    if response=$(curl -s "$url" 2>/dev/null); then
        if echo "$response" | jq . > /dev/null 2>&1; then
            echo -e "${GREEN}✅ PASS${NC} (Valid JSON)"
            return 0
        else
            echo -e "${YELLOW}⚠️  WARN${NC} (Invalid JSON)"
            return 1
        fi
    else
        echo -e "${RED}❌ FAIL${NC} (Connection failed)"
        return 1
    fi
}

echo "🌐 Testing public endpoints..."

# Test UI accessibility
test_endpoint "http://ui.ippan.org/" "UI Homepage"

# Test API health
test_endpoint "http://ui.ippan.org/api/health" "API Health"
test_json_endpoint "http://ui.ippan.org/api/health" "API Health JSON"

# Test API version
test_endpoint "http://ui.ippan.org/api/version" "API Version"
test_json_endpoint "http://ui.ippan.org/api/version" "API Version JSON"

# Test peers endpoint
test_endpoint "http://ui.ippan.org/api/peers" "Peers Endpoint"
test_json_endpoint "http://ui.ippan.org/api/peers" "Peers JSON"

# Test blockchain data endpoints
test_endpoint "http://ui.ippan.org/api/time" "Time Endpoint"
test_json_endpoint "http://ui.ippan.org/api/time" "Time JSON"

# Test block endpoint (may fail if no blocks exist)
echo -n "Testing Block Endpoint... "
if response=$(curl -s -w "%{http_code}" -o /dev/null "http://ui.ippan.org/api/block/1" 2>/dev/null); then
    if [ "$response" = "200" ]; then
        echo -e "${GREEN}✅ PASS${NC} (Block found)"
    elif [ "$response" = "404" ]; then
        echo -e "${YELLOW}⚠️  WARN${NC} (No blocks yet - normal for new network)"
    else
        echo -e "${RED}❌ FAIL${NC} (HTTP $response)"
    fi
else
    echo -e "${RED}❌ FAIL${NC} (Connection failed)"
fi

# Test WebSocket handshake
echo -n "Testing WebSocket Handshake... "
if response=$(curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Host: ui.ippan.org" -H "Origin: https://ui.ippan.org" -H "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" -H "Sec-WebSocket-Version: 13" "http://ui.ippan.org/ws" 2>/dev/null | head -1); then
    if echo "$response" | grep -q "101"; then
        echo -e "${GREEN}✅ PASS${NC} (WebSocket upgrade successful)"
    else
        echo -e "${YELLOW}⚠️  WARN${NC} (WebSocket upgrade failed: $response)"
    fi
else
    echo -e "${RED}❌ FAIL${NC} (WebSocket connection failed)"
fi

echo ""
echo "🔍 Testing local endpoints (if accessible)..."

# Test local gateway
if curl -s http://localhost:8081/health > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Local gateway accessible${NC}"
    test_json_endpoint "http://localhost:8081/api/health" "Local API Health"
    test_json_endpoint "http://localhost:8081/api/version" "Local API Version"
    test_json_endpoint "http://localhost:8081/api/peers" "Local Peers"
else
    echo -e "${YELLOW}⚠️  Local gateway not accessible${NC}"
fi

echo ""
echo "📊 Summary:"
echo "🌐 UI: http://ui.ippan.org/"
echo "🔗 API: http://ui.ippan.org/api/"
echo "📡 WebSocket: ws://ui.ippan.org/ws"
echo ""
echo "🔧 If any tests failed, run: ./fix-gateway.sh"