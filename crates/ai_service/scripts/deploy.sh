#!/bin/bash

# Production deployment script for AI Service

set -euo pipefail

# Configuration
SERVICE_NAME="ippan-ai-service"
VERSION=${1:-"latest"}
ENVIRONMENT=${2:-"production"}
DOCKER_REGISTRY=${3:-"localhost:5000"}

echo "🚀 Deploying $SERVICE_NAME v$VERSION to $ENVIRONMENT"

# Build the service
echo "📦 Building $SERVICE_NAME..."
cargo build --release --package $SERVICE_NAME

# Build Docker image
echo "🐳 Building Docker image..."
docker build -f Dockerfile.production -t $SERVICE_NAME:$VERSION .

# Tag for registry
docker tag $SERVICE_NAME:$VERSION $DOCKER_REGISTRY/$SERVICE_NAME:$VERSION

# Push to registry
echo "📤 Pushing to registry..."
docker push $DOCKER_REGISTRY/$SERVICE_NAME:$VERSION

# Deploy to environment
echo "🚀 Deploying to $ENVIRONMENT..."

case $ENVIRONMENT in
    "production")
        # Production deployment with high availability
        docker-compose -f docker-compose.prod.yml up -d
        ;;
    "staging")
        # Staging deployment
        docker-compose -f docker-compose.staging.yml up -d
        ;;
    "development")
        # Development deployment
        docker-compose -f docker-compose.dev.yml up -d
        ;;
    *)
        echo "❌ Unknown environment: $ENVIRONMENT"
        exit 1
        ;;
esac

# Health check
echo "🏥 Performing health check..."
sleep 10

if curl -f http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Service is healthy"
else
    echo "❌ Service health check failed"
    exit 1
fi

echo "🎉 Deployment completed successfully!"