#!/bin/bash

# IPPAN Load Balancing and Auto-Scaling Setup Script
# This script sets up load balancing and auto-scaling for high availability

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Setup Nginx Load Balancer
setup_nginx_loadbalancer() {
    log_info "Setting up Nginx Load Balancer..."
    
    # Create nginx configuration for load balancing
    cat > /tmp/nginx-loadbalancer.conf << 'EOF'
upstream ippan_backend {
    least_conn;
    server ippan-node-1:3000 max_fails=3 fail_timeout=30s;
    server ippan-node-2:3000 max_fails=3 fail_timeout=30s;
    server ippan-node-3:3000 max_fails=3 fail_timeout=30s;
    keepalive 32;
}

upstream ippan_p2p {
    ip_hash;
    server ippan-node-1:8080;
    server ippan-node-2:8080;
    server ippan-node-3:8080;
    keepalive 32;
}

server {
    listen 80;
    server_name _;
    
    location /api/ {
        proxy_pass http://ippan_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }
    
    location /p2p/ {
        proxy_pass http://ippan_p2p;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
EOF
    
    # Start nginx load balancer
    docker run -d \
        --name ippan-loadbalancer \
        --network ippan_network \
        -p 80:80 \
        -p 443:443 \
        -v /tmp/nginx-loadbalancer.conf:/etc/nginx/conf.d/default.conf \
        nginx:alpine
    
    log_success "Nginx Load Balancer setup completed"
}

# Setup HAProxy Load Balancer
setup_haproxy_loadbalancer() {
    log_info "Setting up HAProxy Load Balancer..."
    
    # Create HAProxy configuration
    cat > /tmp/haproxy.cfg << 'EOF'
global
    daemon
    maxconn 4096
    log stdout local0

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms
    option httplog
    option dontlognull
    option redispatch
    retries 3

frontend ippan_frontend
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/ippan.pem
    redirect scheme https if !{ ssl_fc }
    
    acl is_api path_beg /api/
    acl is_p2p path_beg /p2p/
    
    use_backend ippan_api if is_api
    use_backend ippan_p2p if is_p2p
    default_backend ippan_api

backend ippan_api
    balance roundrobin
    option httpchk GET /api/v1/status
    server ippan-node-1 ippan-node-1:3000 check
    server ippan-node-2 ippan-node-2:3000 check
    server ippan-node-3 ippan-node-3:3000 check

backend ippan_p2p
    balance source
    server ippan-node-1 ippan-node-1:8080 check
    server ippan-node-2 ippan-node-2:8080 check
    server ippan-node-3 ippan-node-3:8080 check

listen stats
    bind *:8404
    stats enable
    stats uri /stats
    stats refresh 5s
    stats admin if TRUE
EOF
    
    # Start HAProxy load balancer
    docker run -d \
        --name ippan-haproxy \
        --network ippan_network \
        -p 80:80 \
        -p 443:443 \
        -p 8404:8404 \
        -v /tmp/haproxy.cfg:/usr/local/etc/haproxy/haproxy.cfg \
        haproxy:latest
    
    log_success "HAProxy Load Balancer setup completed"
}

# Setup Docker Swarm
setup_docker_swarm() {
    log_info "Setting up Docker Swarm..."
    
    # Initialize swarm
    docker swarm init
    
    # Create overlay network
    docker network create --driver overlay --attachable ippan_overlay
    
    # Deploy stack
    docker stack deploy -c deployments/docker-compose.swarm.yml ippan
    
    log_success "Docker Swarm setup completed"
}

# Setup Kubernetes
setup_kubernetes() {
    log_info "Setting up Kubernetes..."
    
    # Apply Kubernetes manifests
    kubectl apply -f deployments/kubernetes/ippan-namespace.yaml
    kubectl apply -f deployments/kubernetes/ippan-deployment.yaml
    kubectl apply -f deployments/kubernetes/ippan-service.yaml
    kubectl apply -f deployments/kubernetes/ippan-hpa.yaml
    kubectl apply -f deployments/kubernetes/ippan-ingress.yaml
    
    log_success "Kubernetes setup completed"
}

# Setup Auto-scaling
setup_autoscaling() {
    log_info "Setting up Auto-scaling..."
    
    # Create auto-scaling configuration
    cat > /tmp/autoscaling.conf << 'EOF'
# Auto-scaling configuration
MIN_REPLICAS=3
MAX_REPLICAS=10
TARGET_CPU=70
TARGET_MEMORY=80
SCALE_UP_COOLDOWN=300
SCALE_DOWN_COOLDOWN=300
EOF
    
    # Apply auto-scaling rules
    if command -v kubectl &> /dev/null; then
        kubectl apply -f deployments/kubernetes/ippan-hpa.yaml
    fi
    
    log_success "Auto-scaling setup completed"
}

# Main setup function
main() {
    log_info "Starting IPPAN load balancing and auto-scaling setup..."
    
    # Choose load balancer type
    echo "Select load balancer type:"
    echo "1) Nginx"
    echo "2) HAProxy"
    echo "3) Docker Swarm"
    echo "4) Kubernetes"
    read -p "Enter choice (1-4): " choice
    
    case $choice in
        1)
            setup_nginx_loadbalancer
            ;;
        2)
            setup_haproxy_loadbalancer
            ;;
        3)
            setup_docker_swarm
            ;;
        4)
            setup_kubernetes
            ;;
        *)
            log_warning "Invalid choice. Setting up Nginx by default."
            setup_nginx_loadbalancer
            ;;
    esac
    
    setup_autoscaling
    
    log_success "IPPAN load balancing and auto-scaling setup completed successfully!"
    echo ""
    echo "🔧 Load Balancer Information:"
    echo "  - HTTP: http://localhost"
    echo "  - HTTPS: https://localhost"
    echo "  - Stats: http://localhost:8404/stats (HAProxy only)"
    echo ""
    echo "📊 Auto-scaling Configuration:"
    echo "  - Min Replicas: 3"
    echo "  - Max Replicas: 10"
    echo "  - Target CPU: 70%"
    echo "  - Target Memory: 80%"
    echo ""
    echo "🔧 Next Steps:"
    echo "  1. Configure health checks"
    echo "  2. Set up monitoring for load balancer"
    echo "  3. Configure SSL certificates"
    echo "  4. Test failover scenarios"
}

main "$@"
