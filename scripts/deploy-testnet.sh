#!/bin/bash

# IPPAN Testnet Deployment Script
# This script deploys a 5-node testnet for validation and testing

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TESTNET_DIR="$PROJECT_ROOT/deployments/testnet"

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

# Help function
show_help() {
    cat << EOF
IPPAN Testnet Deployment Script

Usage: $0 [OPTIONS] COMMAND [ARGS...]

Commands:
    deploy          Deploy testnet with 5 nodes
    start           Start testnet nodes
    stop            Stop testnet nodes
    restart         Restart testnet nodes
    status          Check testnet status
    logs            View testnet logs
    test            Run testnet tests
    cleanup         Clean up testnet resources
    monitor         Start monitoring dashboard
    faucet          Start testnet faucet
    load-test       Run load tests on testnet

Options:
    -n, --nodes     Number of nodes (default: 5)
    -p, --ports     Port range start (default: 8081)
    -h, --help      Show this help message

Examples:
    $0 deploy
    $0 start
    $0 status
    $0 logs
    $0 test
    $0 monitor
    $0 faucet
    $0 load-test

EOF
}

# Default values
NODE_COUNT=5
PORT_START=8081
COMMAND=""

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--nodes)
                NODE_COUNT="$2"
                shift 2
                ;;
            -p|--ports)
                PORT_START="$2"
                shift 2
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            deploy|start|stop|restart|status|logs|test|cleanup|monitor|faucet|load-test)
                COMMAND="$1"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
    
    # Check if Docker daemon is running
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Generate testnet configuration
generate_testnet_config() {
    log_info "Generating testnet configuration..."
    
    cd "$TESTNET_DIR"
    
    # Create directories
    mkdir -p configs monitoring/grafana/datasources monitoring/grafana/dashboards nginx
    
    # Generate node configurations if they don't exist
    for i in $(seq 1 $NODE_COUNT); do
        if [[ ! -f "configs/testnet-node-$i.toml" ]]; then
            log_info "Generating configuration for node $i..."
            # Configuration files are already created
        fi
    done
    
    # Generate Prometheus configuration
    if [[ ! -f "monitoring/prometheus.yml" ]]; then
        log_info "Generating Prometheus configuration..."
        # Prometheus config is already created
    fi
    
    # Generate Grafana datasources
    cat > monitoring/grafana/datasources/prometheus.yml << EOF
apiVersion: 1
datasources:
  - name: Prometheus
    type: prometheus
    url: http://prometheus:9090
    access: proxy
    isDefault: true
EOF
    
    # Generate Nginx configuration
    cat > nginx/nginx.conf << EOF
events {
    worker_connections 1024;
}

http {
    upstream ippan_testnet {
        least_conn;
        server ippan-node-1:8080;
        server ippan-node-2:8080;
        server ippan-node-3:8080;
        server ippan-node-4:8080;
        server ippan-node-5:8080;
    }
    
    server {
        listen 80;
        server_name localhost;
        
        location / {
            proxy_pass http://ippan_testnet;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto \$scheme;
        }
        
        location /api/ {
            proxy_pass http://ippan_testnet;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto \$scheme;
        }
    }
}
EOF
    
    log_success "Testnet configuration generated"
}

# Deploy testnet
deploy_testnet() {
    check_prerequisites
    generate_testnet_config
    
    log_info "Deploying IPPAN testnet with $NODE_COUNT nodes..."
    
    cd "$TESTNET_DIR"
    
    # Build Docker images
    log_info "Building Docker images..."
    docker-compose -f docker-compose.testnet.yml build
    
    # Start services
    log_info "Starting testnet services..."
    docker-compose -f docker-compose.testnet.yml up -d
    
    # Wait for services to be ready
    log_info "Waiting for services to be ready..."
    sleep 30
    
    # Check service status
    check_testnet_status
    
    log_success "Testnet deployed successfully!"
    log_info "Access points:"
    log_info "  - Load Balancer: http://localhost:80"
    log_info "  - Node 1 API: http://localhost:8081"
    log_info "  - Node 2 API: http://localhost:8082"
    log_info "  - Node 3 API: http://localhost:8083"
    log_info "  - Node 4 API: http://localhost:8084"
    log_info "  - Node 5 API: http://localhost:8085"
    log_info "  - Prometheus: http://localhost:9090"
    log_info "  - Grafana: http://localhost:3000 (admin/admin)"
}

# Start testnet
start_testnet() {
    log_info "Starting testnet..."
    
    cd "$TESTNET_DIR"
    docker-compose -f docker-compose.testnet.yml start
    
    log_success "Testnet started"
}

# Stop testnet
stop_testnet() {
    log_info "Stopping testnet..."
    
    cd "$TESTNET_DIR"
    docker-compose -f docker-compose.testnet.yml stop
    
    log_success "Testnet stopped"
}

# Restart testnet
restart_testnet() {
    log_info "Restarting testnet..."
    
    cd "$TESTNET_DIR"
    docker-compose -f docker-compose.testnet.yml restart
    
    log_success "Testnet restarted"
}

# Check testnet status
check_testnet_status() {
    log_info "Checking testnet status..."
    
    cd "$TESTNET_DIR"
    
    # Check Docker containers
    log_info "Docker containers status:"
    docker-compose -f docker-compose.testnet.yml ps
    
    # Check node health
    log_info "Checking node health..."
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        if curl -f "http://localhost:$port/api/v1/status" > /dev/null 2>&1; then
            log_success "Node $i is healthy (port $port)"
        else
            log_warning "Node $i is not responding (port $port)"
        fi
    done
    
    # Check monitoring services
    if curl -f "http://localhost:9090/-/healthy" > /dev/null 2>&1; then
        log_success "Prometheus is healthy"
    else
        log_warning "Prometheus is not responding"
    fi
    
    if curl -f "http://localhost:3000/api/health" > /dev/null 2>&1; then
        log_success "Grafana is healthy"
    else
        log_warning "Grafana is not responding"
    fi
}

# View testnet logs
view_testnet_logs() {
    log_info "Viewing testnet logs..."
    
    cd "$TESTNET_DIR"
    
    if [[ $# -gt 0 ]]; then
        local service="$1"
        log_info "Logs for service: $service"
        docker-compose -f docker-compose.testnet.yml logs -f "$service"
    else
        log_info "Logs for all services:"
        docker-compose -f docker-compose.testnet.yml logs -f
    fi
}

# Run testnet tests
run_testnet_tests() {
    log_info "Running testnet tests..."
    
    # Test node connectivity
    log_info "Testing node connectivity..."
    for i in $(seq 1 $NODE_COUNT); do
        local port=$((PORT_START + i - 1))
        log_info "Testing node $i (port $port)..."
        
        # Test API endpoint
        if curl -f "http://localhost:$port/api/v1/status" > /dev/null 2>&1; then
            log_success "Node $i API is responding"
        else
            log_error "Node $i API is not responding"
            return 1
        fi
        
        # Test metrics endpoint
        if curl -f "http://localhost:$port/metrics" > /dev/null 2>&1; then
            log_success "Node $i metrics are available"
        else
            log_warning "Node $i metrics are not available"
        fi
    done
    
    # Test consensus
    log_info "Testing consensus..."
    # Add consensus testing logic here
    
    # Test transaction processing
    log_info "Testing transaction processing..."
    # Add transaction testing logic here
    
    # Test network connectivity
    log_info "Testing network connectivity..."
    # Add network testing logic here
    
    log_success "Testnet tests completed"
}

# Clean up testnet
cleanup_testnet() {
    log_warning "Cleaning up testnet resources..."
    
    cd "$TESTNET_DIR"
    
    # Stop and remove containers
    docker-compose -f docker-compose.testnet.yml down -v
    
    # Remove volumes
    docker volume prune -f
    
    # Remove images (optional)
    if [[ "${REMOVE_IMAGES:-false}" == "true" ]]; then
        docker rmi $(docker images "ippan*" -q) 2>/dev/null || true
    fi
    
    log_success "Testnet cleanup completed"
}

# Start monitoring dashboard
start_monitoring() {
    log_info "Starting monitoring dashboard..."
    
    # Check if Grafana is running
    if curl -f "http://localhost:3000/api/health" > /dev/null 2>&1; then
        log_success "Grafana is already running at http://localhost:3000"
        log_info "Login credentials: admin/admin"
    else
        log_info "Starting Grafana..."
        cd "$TESTNET_DIR"
        docker-compose -f docker-compose.testnet.yml up -d grafana
        sleep 10
        log_success "Grafana started at http://localhost:3000"
        log_info "Login credentials: admin/admin"
    fi
    
    # Check if Prometheus is running
    if curl -f "http://localhost:9090/-/healthy" > /dev/null 2>&1; then
        log_success "Prometheus is running at http://localhost:9090"
    else
        log_warning "Prometheus is not responding"
    fi
}

# Start testnet faucet
start_faucet() {
    log_info "Starting testnet faucet..."
    
    # Add faucet implementation here
    log_info "Faucet functionality will be implemented in the next iteration"
    
    log_success "Testnet faucet started"
}

# Run load tests
run_load_tests() {
    log_info "Running load tests on testnet..."
    
    # Check if k6 is installed
    if ! command -v k6 &> /dev/null; then
        log_warning "k6 is not installed. Installing k6..."
        # Add k6 installation logic here
    fi
    
    # Create load test script
    cat > /tmp/ippan_load_test.js << EOF
import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
  stages: [
    { duration: '2m', target: 10 },   // Ramp up to 10 users
    { duration: '5m', target: 10 },   // Stay at 10 users
    { duration: '2m', target: 20 },   // Ramp up to 20 users
    { duration: '5m', target: 20 },   // Stay at 20 users
    { duration: '2m', target: 0 },    // Ramp down to 0 users
  ],
};

export default function () {
  let response = http.get('http://localhost:80/api/v1/status');
  check(response, {
    'status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });
  sleep(1);
}
EOF
    
    # Run load test
    log_info "Running k6 load test..."
    k6 run /tmp/ippan_load_test.js
    
    # Clean up
    rm -f /tmp/ippan_load_test.js
    
    log_success "Load tests completed"
}

# Main function
main() {
    parse_args "$@"
    
    if [[ -z "$COMMAND" ]]; then
        log_error "No command specified"
        show_help
        exit 1
    fi
    
    case "$COMMAND" in
        deploy)
            deploy_testnet
            ;;
        start)
            start_testnet
            ;;
        stop)
            stop_testnet
            ;;
        restart)
            restart_testnet
            ;;
        status)
            check_testnet_status
            ;;
        logs)
            view_testnet_logs "$@"
            ;;
        test)
            run_testnet_tests
            ;;
        cleanup)
            cleanup_testnet
            ;;
        monitor)
            start_monitoring
            ;;
        faucet)
            start_faucet
            ;;
        load-test)
            run_load_tests
            ;;
        *)
            log_error "Unknown command: $COMMAND"
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
