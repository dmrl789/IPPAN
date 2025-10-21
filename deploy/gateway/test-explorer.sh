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

API_BASE_URL="${API_BASE_URL:-https://api.ippan.org}"
UI_BASE_URL="${UI_BASE_URL:-https://ui.ippan.org}"
WS_URL="${WS_URL:-wss://api.ippan.org/ws}"

echo "🌐 Testing public endpoints (API_BASE_URL=${API_BASE_URL})..."

# Test UI accessibility (non-fatal if domain changed)
test_endpoint "${UI_BASE_URL}/" "UI Homepage" || true

# Test API health
test_endpoint "${API_BASE_URL}/health" "API Health"
test_json_endpoint "${API_BASE_URL}/health" "API Health JSON"

# Test API version
test_endpoint "${API_BASE_URL}/version" "API Version"
test_json_endpoint "${API_BASE_URL}/version" "API Version JSON"

# Test peers endpoint
test_endpoint "${API_BASE_URL}/peers" "Peers Endpoint"
test_json_endpoint "${API_BASE_URL}/peers" "Peers JSON"

# Test blockchain data endpoints
test_endpoint "${API_BASE_URL}/time" "Time Endpoint"
test_json_endpoint "${API_BASE_URL}/time" "Time JSON"

# Test block endpoint (may fail if no blocks exist)
echo -n "Testing Block Endpoint... "
if response=$(curl -s -w "%{http_code}" -o /dev/null "${API_BASE_URL}/block/1" 2>/dev/null); then
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
WS_HTTP_URL=$(echo "$WS_URL" | sed 's/^wss:/https:/; s/^ws:/http:/')
ORIGIN_URL="${UI_BASE_URL}"
HOST_HEADER=$(echo "$WS_HTTP_URL" | sed -E 's#^https?://([^/]+)/?.*$#\1#')
if response=$(curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Host: ${HOST_HEADER}" -H "Origin: ${ORIGIN_URL}" -H "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" -H "Sec-WebSocket-Version: 13" "${WS_HTTP_URL}" 2>/dev/null | head -1); then
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
echo "🌐 UI: ${UI_BASE_URL}/"
echo "🔗 API: ${API_BASE_URL}/"
echo "📡 WebSocket: ${WS_URL}"
echo ""
echo "🔧 If any tests failed, run: ./fix-gateway.sh"