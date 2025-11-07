#!/bin/bash

# Deploy IPPAN Node 2 to Server 2 (135.181.145.174)
# This script deploys only Node 2

set -e

echo "🚀 Deploying IPPAN Node 2 to Server 2..."

# Resolve Docker image tag
IMAGE_TAG="${IMAGE_TAG:-${GITHUB_SHA:-latest}}"
export IMAGE_TAG
echo "🖼️ Using Docker image tag: ${IMAGE_TAG}"

# Check if Docker and Docker Compose are available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose is not available. Please install Docker Compose first."
    exit 1
fi

# Create data directories
echo "📁 Creating data directories..."
mkdir -p ./data/node2

# Pull latest images
echo "📥 Pulling latest Docker images..."
docker pull "ghcr.io/dmrl789/ippan/ippan-node:${IMAGE_TAG}"
docker-compose -f docker-compose.production.yml pull

# Stop existing containers if running
echo "🛑 Stopping existing containers..."
docker-compose -f docker-compose.production.yml down --remove-orphans || true

# Start Node 2
echo "🏃 Starting IPPAN Node 2..."
docker-compose -f docker-compose.production.yml up -d

# Wait for services to be ready
echo "⏳ Waiting for services to start..."
sleep 10

# Check service status
echo "📊 Checking service status..."
docker-compose -f docker-compose.production.yml ps

# Health checks
echo "🏥 Running health checks..."

# Check Node 2
if curl -f -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Node 2 is responding on port 8080"
else
    echo "⚠️  Node 2 health check failed"
    exit 1
fi

# Check P2P connectivity
echo "🔗 Checking P2P connectivity..."
if ss -ltnp | grep :4001 > /dev/null; then
    echo "✅ P2P port 4001 is listening"
else
    echo "⚠️  P2P port 4001 is not listening"
fi

echo ""
echo "🎉 Deployment complete!"
echo ""
echo "📋 Service URLs:"
echo "   Node 2:   http://135.181.145.174:8080"
echo "   P2P:      135.181.145.174:4001"
echo ""
echo "🔍 To check logs: docker-compose -f docker-compose.production.yml logs -f"
echo "🛑 To stop:       docker-compose -f docker-compose.production.yml down"