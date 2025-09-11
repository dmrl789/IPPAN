#!/bin/bash

# IPPAN Staging Deployment Script
# This script deploys IPPAN to staging environment and runs integration tests

set -e

# Configuration
PROJECT_NAME="ippan"
ENVIRONMENT="staging"
DOCKER_COMPOSE_FILE="deployments/staging/docker-compose.staging.yml"
ENV_FILE=".env.staging"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi
    
    log_success "All prerequisites are met."
}

# Create environment file
create_environment_file() {
    log_info "Creating staging environment file..."
    
    if [ ! -f "$ENV_FILE" ]; then
        cat > "$ENV_FILE" << EOF
# IPPAN Staging Environment Configuration

# Database
POSTGRES_DB=ippan_staging
POSTGRES_USER=ippan
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-staging123}

# Redis
REDIS_PASSWORD=${REDIS_PASSWORD:-staging123}

# JWT
JWT_SECRET=${JWT_SECRET:-staging-secret-key}

# Grafana
GRAFANA_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-staging123}
GRAFANA_SECRET_KEY=${GRAFANA_SECRET_KEY:-staging-secret-key}

# Backup
BACKUP_ENCRYPTION_KEY=${BACKUP_ENCRYPTION_KEY:-staging-backup-key}

# SMTP (for alerts)
SMTP_PASSWORD=${SMTP_PASSWORD:-}
EMAIL_SMTP_HOST=${EMAIL_SMTP_HOST:-smtp.gmail.com}
EMAIL_SMTP_PORT=${EMAIL_SMTP_PORT:-587}
EMAIL_USERNAME=${EMAIL_USERNAME:-alerts@ippan.network}
EMAIL_PASSWORD=${EMAIL_PASSWORD:-}

# Slack (for alerts)
SLACK_WEBHOOK_URL=${SLACK_WEBHOOK_URL:-}

# Network
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000
IPPAN_P2P_PORT=8080

# Security (relaxed for staging)
IPPAN_ENABLE_TLS=false
IPPAN_ENABLE_MUTUAL_AUTH=false

# Performance (reduced for staging)
IPPAN_MAX_CONNECTIONS=1000
IPPAN_THREAD_POOL_SIZE=8
IPPAN_CACHE_SIZE=1073741824
IPPAN_MEMORY_POOL_SIZE=536870912

# Monitoring
PROMETHEUS_RETENTION=7d
GRAFANA_ADMIN_USER=admin
ALERTMANAGER_CONFIG_FILE=/etc/alertmanager/alertmanager.yml

# Logging
RUST_LOG=debug
LOG_LEVEL=debug
LOG_FORMAT=json

# Staging flags
NODE_ENV=staging
IPPAN_ENVIRONMENT=staging
IPPAN_ENABLE_METRICS=true
IPPAN_ENABLE_HEALTH_CHECKS=true
IPPAN_ENABLE_TEST_MODE=true
IPPAN_ENABLE_DEBUG_ENDPOINTS=true
EOF
        log_success "Staging environment file created: $ENV_FILE"
    else
        log_warning "Staging environment file already exists: $ENV_FILE"
    fi
}

# Build Docker images
build_docker_images() {
    log_info "Building Docker images for staging..."
    
    # Build main IPPAN image with staging tag
    docker build -f Dockerfile.production -t ippan/ippan:staging .
    
    # Build test runner image
    cat > Dockerfile.test-runner << 'EOF'
FROM python:3.9-alpine

# Install dependencies
RUN apk add --no-cache curl bash

# Copy test scripts
COPY tests/ /tests/
RUN chmod +x /tests/*.sh

# Set working directory
WORKDIR /tests

# Default command
CMD ["/tests/run-integration-tests.sh"]
EOF
    
    docker build -f Dockerfile.test-runner -t ippan/test-runner:latest .
    
    # Pull monitoring images
    docker pull prom/prometheus:latest
    docker pull grafana/grafana:latest
    docker pull prom/alertmanager:latest
    docker pull fluent/fluentd:v1.16-debian-1
    docker pull nginx:alpine
    
    log_success "Docker images built successfully."
}

# Deploy services
deploy_services() {
    log_info "Deploying IPPAN staging services..."
    
    # Stop existing services
    docker-compose -f "$DOCKER_COMPOSE_FILE" --env-file "$ENV_FILE" down
    
    # Start services
    docker-compose -f "$DOCKER_COMPOSE_FILE" --env-file "$ENV_FILE" up -d
    
    log_success "IPPAN staging services deployed successfully."
}

# Wait for services to be ready
wait_for_services() {
    log_info "Waiting for services to be ready..."
    
    # Wait for IPPAN node
    log_info "Waiting for IPPAN staging node..."
    timeout 300 bash -c 'until curl -f http://localhost:3000/api/v1/status; do sleep 5; done'
    
    # Wait for Prometheus
    log_info "Waiting for Prometheus..."
    timeout 300 bash -c 'until curl -f http://localhost:9090/-/healthy; do sleep 5; done'
    
    # Wait for Grafana
    log_info "Waiting for Grafana..."
    timeout 300 bash -c 'until curl -f http://localhost:3001/api/health; do sleep 5; done'
    
    log_success "All services are ready."
}

# Run integration tests
run_integration_tests() {
    log_info "Running integration tests..."
    
    # Run test runner container
    docker-compose -f "$DOCKER_COMPOSE_FILE" --env-file "$ENV_FILE" run --rm ippan-staging-test-runner
    
    # Check test results
    if [ -f "staging_test_results/integration-test-report.md" ]; then
        log_success "Integration tests completed successfully"
        
        # Display test summary
        echo ""
        echo "📊 Integration Test Summary:"
        grep -E "(✅ PASSED|❌ FAILED)" staging_test_results/integration-test-report.md | head -10
    else
        log_error "Integration tests failed or report not found"
        return 1
    fi
}

# Verify deployment
verify_deployment() {
    log_info "Verifying staging deployment..."
    
    # Check IPPAN node health
    if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
        log_success "IPPAN staging node is healthy"
    else
        log_error "IPPAN staging node health check failed"
        exit 1
    fi
    
    # Check Prometheus
    if curl -f http://localhost:9090/-/healthy > /dev/null 2>&1; then
        log_success "Prometheus is healthy"
    else
        log_error "Prometheus health check failed"
        exit 1
    fi
    
    # Check Grafana
    if curl -f http://localhost:3001/api/health > /dev/null 2>&1; then
        log_success "Grafana is healthy"
    else
        log_error "Grafana health check failed"
        exit 1
    fi
    
    # Check Nginx
    if curl -f http://localhost/health > /dev/null 2>&1; then
        log_success "Nginx is healthy"
    else
        log_error "Nginx health check failed"
        exit 1
    fi
    
    log_success "Staging deployment verification completed successfully."
}

# Show deployment information
show_deployment_info() {
    log_info "Staging Deployment Information:"
    echo ""
    echo "🌐 IPPAN Staging Node API: http://localhost:3000"
    echo "📊 Prometheus: http://localhost:9090"
    echo "📈 Grafana: http://localhost:3001"
    echo "🔔 AlertManager: http://localhost:9093"
    echo "🌍 Load Balancer: http://localhost (HTTP) / https://localhost (HTTPS)"
    echo ""
    echo "📋 Useful Commands:"
    echo "  View logs: docker-compose -f $DOCKER_COMPOSE_FILE logs -f"
    echo "  Stop services: docker-compose -f $DOCKER_COMPOSE_FILE down"
    echo "  Restart services: docker-compose -f $DOCKER_COMPOSE_FILE restart"
    echo "  Run tests: docker-compose -f $DOCKER_COMPOSE_FILE run --rm ippan-staging-test-runner"
    echo ""
    echo "🔐 Staging Configuration:"
    echo "  - Environment file: $ENV_FILE"
    echo "  - Configuration: config/staging.toml"
    echo "  - Test results: staging_test_results/"
    echo "  - Logs: staging_logs/"
    echo ""
    echo "📊 Monitoring:"
    echo "  - Grafana admin password: Check $ENV_FILE"
    echo "  - Prometheus metrics: http://localhost:9090/metrics"
    echo "  - Integration test report: staging_test_results/integration-test-report.md"
    echo ""
}

# Main deployment function
main() {
    log_info "Starting IPPAN staging deployment..."
    
    check_prerequisites
    create_environment_file
    build_docker_images
    deploy_services
    wait_for_services
    verify_deployment
    run_integration_tests
    show_deployment_info
    
    log_success "IPPAN staging deployment completed successfully! 🚀"
}

# Run main function
main "$@"
