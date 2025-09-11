#!/bin/bash

# IPPAN Monitoring Setup Script
# This script sets up comprehensive monitoring for IPPAN production

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

# Setup Prometheus
setup_prometheus() {
    log_info "Setting up Prometheus..."
    
    # Create Prometheus data directory
    mkdir -p /var/lib/prometheus
    chown -R 65534:65534 /var/lib/prometheus
    
    # Start Prometheus
    docker run -d \
        --name prometheus \
        --network ippan_network \
        -p 9090:9090 \
        -v $(pwd)/deployments/monitoring/prometheus-production.yml:/etc/prometheus/prometheus.yml \
        -v /var/lib/prometheus:/prometheus \
        prom/prometheus:latest \
        --config.file=/etc/prometheus/prometheus.yml \
        --storage.tsdb.path=/prometheus \
        --web.console.libraries=/etc/prometheus/console_libraries \
        --web.console.templates=/etc/prometheus/consoles \
        --storage.tsdb.retention.time=30d \
        --web.enable-lifecycle
    
    log_success "Prometheus setup completed"
}

# Setup Grafana
setup_grafana() {
    log_info "Setting up Grafana..."
    
    # Create Grafana data directory
    mkdir -p /var/lib/grafana
    chown -R 472:472 /var/lib/grafana
    
    # Start Grafana
    docker run -d \
        --name grafana \
        --network ippan_network \
        -p 3001:3001 \
        -v /var/lib/grafana:/var/lib/grafana \
        -v $(pwd)/deployments/monitoring/grafana-production.yml:/etc/grafana/grafana.ini \
        -v $(pwd)/deployments/monitoring/grafana-dashboards:/etc/grafana/provisioning/dashboards \
        -v $(pwd)/deployments/monitoring/grafana-datasource.yml:/etc/grafana/provisioning/datasources \
        -e GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-admin123} \
        grafana/grafana:latest
    
    log_success "Grafana setup completed"
}

# Setup AlertManager
setup_alertmanager() {
    log_info "Setting up AlertManager..."
    
    # Create AlertManager data directory
    mkdir -p /var/lib/alertmanager
    chown -R 65534:65534 /var/lib/alertmanager
    
    # Start AlertManager
    docker run -d \
        --name alertmanager \
        --network ippan_network \
        -p 9093:9093 \
        -v $(pwd)/deployments/monitoring/alertmanager-production.yml:/etc/alertmanager/alertmanager.yml \
        -v /var/lib/alertmanager:/alertmanager \
        prom/alertmanager:latest \
        --config.file=/etc/alertmanager/alertmanager.yml \
        --storage.path=/alertmanager \
        --web.external-url=http://localhost:9093
    
    log_success "AlertManager setup completed"
}

# Setup Fluentd
setup_fluentd() {
    log_info "Setting up Fluentd..."
    
    # Create Fluentd data directory
    mkdir -p /var/log/fluentd
    chown -R 100:101 /var/log/fluentd
    
    # Start Fluentd
    docker run -d \
        --name fluentd \
        --network ippan_network \
        -p 24224:24224 \
        -v $(pwd)/deployments/monitoring/fluentd.conf:/fluentd/etc/fluent.conf \
        -v /var/log/ippan:/var/log/ippan:ro \
        -v /var/log/fluentd:/var/log/fluentd \
        fluent/fluentd:v1.16-debian-1
    
    log_success "Fluentd setup completed"
}

# Main setup function
main() {
    log_info "Starting IPPAN monitoring setup..."
    
    setup_prometheus
    setup_grafana
    setup_alertmanager
    setup_fluentd
    
    log_success "IPPAN monitoring setup completed successfully!"
    echo ""
    echo "📊 Monitoring URLs:"
    echo "  - Prometheus: http://localhost:9090"
    echo "  - Grafana: http://localhost:3001 (admin/admin123)"
    echo "  - AlertManager: http://localhost:9093"
    echo ""
    echo "🔧 Next Steps:"
    echo "  1. Configure Grafana datasources"
    echo "  2. Import dashboard templates"
    echo "  3. Set up alerting rules"
    echo "  4. Configure notification channels"
}

main "$@"
