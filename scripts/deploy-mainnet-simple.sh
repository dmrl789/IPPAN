#!/bin/bash

# IPPAN Mainnet Deployment Script (Simplified)
set -e

log_info() { echo -e "\033[0;34m[INFO]\033[0m $1"; }
log_success() { echo -e "\033[0;32m[SUCCESS]\033[0m $1"; }
log_error() { echo -e "\033[0;31m[ERROR]\033[0m $1"; }

# Configuration
DOCKER_COMPOSE_FILE="deployments/mainnet/docker-compose.mainnet.yml"
ENV_FILE=".env.mainnet"

# Main deployment function
main() {
    log_info "Starting IPPAN mainnet deployment..."
    
    # Check prerequisites
    log_info "Checking prerequisites..."
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
    
    # Create environment file if it doesn't exist
    if [ ! -f "$ENV_FILE" ]; then
        log_info "Creating mainnet environment file..."
        cat > "$ENV_FILE" << EOF
# IPPAN Mainnet Environment
POSTGRES_DB=ippan_mainnet
POSTGRES_USER=ippan
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-mainnet123}
REDIS_PASSWORD=${REDIS_PASSWORD:-mainnet123}
JWT_SECRET=${JWT_SECRET:-mainnet-jwt-secret}
GRAFANA_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-mainnet123}
NODE_ENV=production
IPPAN_ENVIRONMENT=mainnet
RUST_LOG=info
EOF
    fi
    
    # Build and deploy
    log_info "Building Docker images..."
    docker build -f Dockerfile.production -t ippan/ippan:latest .
    
    log_info "Deploying mainnet services..."
    docker-compose -f "$DOCKER_COMPOSE_FILE" --env-file "$ENV_FILE" up -d
    
    # Wait for services
    log_info "Waiting for services to be ready..."
    sleep 60
    
    # Verify deployment
    log_info "Verifying deployment..."
    if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
        log_success "IPPAN mainnet node is healthy"
    else
        log_error "IPPAN mainnet node health check failed"
        exit 1
    fi
    
    log_success "IPPAN mainnet deployment completed successfully! 🚀"
    echo ""
    echo "🎉 IPPAN Mainnet is now LIVE!"
    echo ""
    echo "Services:"
    echo "  - IPPAN API: http://localhost:3000"
    echo "  - Prometheus: http://localhost:9090"
    echo "  - Grafana: http://localhost:3001"
    echo ""
    echo "Commands:"
    echo "  - View logs: docker-compose -f $DOCKER_COMPOSE_FILE logs -f"
    echo "  - Stop: docker-compose -f $DOCKER_COMPOSE_FILE down"
    echo ""
}

# Run main function
main "$@"
