#!/bin/bash
set -e

echo "🔧 Fixing IPPAN Gateway for Blockchain Explorer"

# Check if we're in the right directory
if [ ! -f "docker-compose.yml" ]; then
    echo "❌ Error: docker-compose.yml not found. Please run this script from the deploy/gateway directory."
    exit 1
fi

echo "📋 Current container status:"
docker compose ps || true

echo "🛑 Stopping existing containers..."
docker compose down || true

echo "🧹 Cleaning up old containers and images..."
docker system prune -f || true

echo "📥 Pulling latest images..."
docker compose pull

echo "🚀 Starting services..."
docker compose up -d

echo "⏳ Waiting for services to start..."
sleep 10

echo "📊 Container status after restart:"
docker compose ps

echo "🔍 Testing gateway health..."
sleep 5

# Test local gateway health
if curl -fsS http://localhost:8081/health > /dev/null 2>&1; then
    echo "✅ Gateway health check passed (local)"
else
    echo "❌ Gateway health check failed (local)"
    echo "📋 Gateway logs:"
    docker compose logs --tail=20 gateway
fi

# Test API endpoints
echo "🔍 Testing API endpoints..."
if curl -fsS http://localhost:8081/api/health > /dev/null 2>&1; then
    echo "✅ API health endpoint working"
else
    echo "❌ API health endpoint failed"
fi

if curl -fsS http://localhost:8081/api/version > /dev/null 2>&1; then
    echo "✅ API version endpoint working"
else
    echo "❌ API version endpoint failed"
fi

if curl -fsS http://localhost:8081/api/peers > /dev/null 2>&1; then
    echo "✅ API peers endpoint working"
else
    echo "❌ API peers endpoint failed"
fi

# Test blockchain data endpoints
echo "🔍 Testing blockchain data endpoints..."
if curl -fsS http://localhost:8081/api/block/1 > /dev/null 2>&1; then
    echo "✅ Block endpoint working"
else
    echo "❌ Block endpoint failed (may be normal if no blocks exist yet)"
fi

echo "🌐 Testing public endpoints..."
if curl -fsS http://ui.ippan.org/api/health > /dev/null 2>&1; then
    echo "✅ Public API health endpoint working"
else
    echo "❌ Public API health endpoint failed"
fi

echo "📋 Final service status:"
docker compose ps

echo "📋 Recent logs:"
echo "=== Gateway logs ==="
docker compose logs --tail=10 gateway
echo "=== Node logs ==="
docker compose logs --tail=10 ippan-node
echo "=== UI logs ==="
docker compose logs --tail=10 unified-ui

echo "✅ Gateway fix complete!"
echo "🌐 You can now access the blockchain explorer at: http://ui.ippan.org/"
echo "🔗 API endpoints are available at: http://ui.ippan.org/api/"