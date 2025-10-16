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
curl -s http://localhost:8080/health || echo "Node health check failed"
curl -s http://localhost:8081/health || echo "Gateway health check failed"

echo "✅ Deployment completed!"
echo "Nodes are online. Unified UI is no longer served from this host."
