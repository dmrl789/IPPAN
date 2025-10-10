#!/bin/bash
set -euo pipefail

echo "ðŸš€ Deploying IPPAN Unified UI..."

# Stop any existing containers
docker-compose down 2>/dev/null || true

# Free up ports
echo "Freeing up ports..."
lsof -ti:80,443,8080,8081,9000,3001 | xargs -r kill -9 2>/dev/null || true

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
curl -s http://localhost:3001 || echo "UI health check failed"

echo "âœ… Deployment completed!"
echo "UI should be available at: http://188.245.97.41"
echo "API should be available at: http://"
