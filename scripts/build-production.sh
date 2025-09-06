#!/bin/bash

# IPPAN Production Build Script
set -e

echo "🚀 Building IPPAN for production..."

# Build frontend
echo "📦 Building frontend..."
cd apps/unified-ui
npm ci --only=production
npm run build
cd ../..

# Build Rust backend
echo "🦀 Building Rust backend..."
cargo build --release --bin ippan

# Build Docker image
echo "🐳 Building Docker image..."
docker build -f Dockerfile.production -t ippan:latest .

# Tag for registry
docker tag ippan:latest ippan:$(git rev-parse --short HEAD)
docker tag ippan:latest ippan:production

echo "✅ Production build completed!"
echo "📋 Next steps:"
echo "   1. Push to registry: docker push ippan:latest"
echo "   2. Deploy to Kubernetes: kubectl apply -f deployments/kubernetes/"
echo "   3. Monitor deployment: kubectl get pods -l app=ippan-node"
