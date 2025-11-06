#!/bin/bash
set -euo pipefail

echo "🚀 Deploying IPPAN nodes (Unified UI disabled)..."

# Stop any existing containers
docker-compose down 2>/dev/null || true

# Free up ports
echo "Freeing up ports..."
lsof -ti:8080,8081,9000 | xargs -r kill -9 2>/dev/null || true

# Pull latest images
echo "Pulling latest images..."
docker-compose pull

# Start services
echo "Starting services..."
docker-compose up -d

# Wait for services to start
echo "Waiting for services to start..."
sleep 15

# Check status
echo "Checking service status..."
docker-compose ps

# Test endpoints
echo "Testing endpoints..."

check_endpoint() {
  local url="$1"
  local label="$2"

  if curl -fsS --max-time 10 "$url" > /dev/null; then
    echo "✅ ${label} responded successfully"
  else
    echo "❌ ${label} check failed (${url})"
    return 1
  fi
}

check_endpoint "http://localhost:8080/health" "Node health"
check_endpoint "http://localhost:8081/health" "Gateway health"

echo "✅ Deployment completed!"
echo "Nodes are online. Unified UI is no longer served from this host."
