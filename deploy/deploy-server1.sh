#!/bin/bash

# Deploy IPPAN Full Stack to Server 1 (188.245.97.41)
# This script deploys UI + Gateway + Node 1 + Load Balancer

set -e

echo "🚀 Deploying IPPAN Full Stack to Server 1..."

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
mkdir -p ./data/node1

# Pull latest images
echo "📥 Pulling latest Docker images..."
docker-compose -f docker-compose.full-stack.yml pull

# Stop existing containers if running
echo "🛑 Stopping existing containers..."
docker-compose -f docker-compose.full-stack.yml down --remove-orphans || true

# Start the full stack
echo "🏃 Starting IPPAN Full Stack..."
docker-compose -f docker-compose.full-stack.yml up -d

# Wait for services to be ready
echo "⏳ Waiting for services to start..."
sleep 10

# Check service status
echo "📊 Checking service status..."
docker-compose -f docker-compose.full-stack.yml ps

# Health checks
echo "🏥 Running health checks..."

# Check UI
if curl -f -s http://localhost:3001 > /dev/null; then
    echo "✅ UI is responding on port 3001"
else
    echo "⚠️  UI health check failed"
fi

# Check Node 1
if curl -f -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Node 1 is responding on port 8080"
else
    echo "⚠️  Node 1 health check failed"
fi

# Check Gateway
if curl -f -s http://localhost:8081 > /dev/null 2>&1; then
    echo "✅ Gateway is responding on port 8081"
else
    echo "⚠️  Gateway health check failed"
fi

# Check Nginx
if curl -f -s http://localhost:80 > /dev/null 2>&1; then
    echo "✅ Nginx load balancer is responding on port 80"
else
    echo "⚠️  Nginx health check failed"
fi

echo ""
echo "🎉 Deployment complete!"
echo ""
echo "📋 Service URLs:"
echo "   UI:       http://188.245.97.41:3001"
echo "   Node 1:   http://188.245.97.41:8080"
echo "   Gateway:  http://188.245.97.41:8081"
echo "   Nginx:    http://188.245.97.41:80"
echo ""
echo "🔍 To check logs: docker-compose -f docker-compose.full-stack.yml logs -f"
echo "🛑 To stop:       docker-compose -f docker-compose.full-stack.yml down"