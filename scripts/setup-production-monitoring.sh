#!/bin/bash

# IPPAN Production Monitoring Setup Script
# This script sets up the complete production monitoring stack for IPPAN blockchain

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
MONITORING_DIR="deployments/monitoring"
BACKUP_DIR="backups/monitoring"
LOG_FILE="setup-monitoring.log"

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}" | tee -a "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}" | tee -a "$LOG_FILE"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}" | tee -a "$LOG_FILE"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed. Please install Docker first."
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose is not installed. Please install Docker Compose first."
    fi
    
    # Check if monitoring directory exists
    if [ ! -d "$MONITORING_DIR" ]; then
        error "Monitoring directory $MONITORING_DIR does not exist."
    fi
    
    log "Prerequisites check passed"
}

# Create backup directory
create_backup_dir() {
    log "Creating backup directory..."
    mkdir -p "$BACKUP_DIR"
    log "Backup directory created: $BACKUP_DIR"
}

# Backup existing configuration
backup_existing_config() {
    log "Backing up existing configuration..."
    
    if [ -d "$MONITORING_DIR" ]; then
        cp -r "$MONITORING_DIR" "$BACKUP_DIR/monitoring-$(date +%Y%m%d-%H%M%S)"
        log "Existing configuration backed up"
    fi
}

# Generate configuration files
generate_config_files() {
    log "Generating configuration files..."
    
    # Generate Prometheus configuration
    if [ ! -f "$MONITORING_DIR/prometheus-production.yml" ]; then
        error "Prometheus configuration file not found: $MONITORING_DIR/prometheus-production.yml"
    fi
    
    # Generate AlertManager configuration
    if [ ! -f "$MONITORING_DIR/alertmanager-production.yml" ]; then
        error "AlertManager configuration file not found: $MONITORING_DIR/alertmanager-production.yml"
    fi
    
    # Generate Grafana configuration
    if [ ! -f "$MONITORING_DIR/grafana-production.yml" ]; then
        error "Grafana configuration file not found: $MONITORING_DIR/grafana-production.yml"
    fi
    
    # Generate Docker Compose file
    if [ ! -f "$MONITORING_DIR/docker-compose-production.yml" ]; then
        error "Docker Compose file not found: $MONITORING_DIR/docker-compose-production.yml"
    fi
    
    log "Configuration files validated"
}

# Create necessary directories
create_directories() {
    log "Creating necessary directories..."
    
    # Create Grafana directories
    mkdir -p "$MONITORING_DIR/dashboards"
    mkdir -p "$MONITORING_DIR/datasources"
    mkdir -p "$MONITORING_DIR/alerting"
    
    # Create log directories
    mkdir -p "/var/log/grafana"
    mkdir -p "/var/log/prometheus"
    mkdir -p "/var/log/alertmanager"
    
    log "Directories created"
}

# Set up Grafana dashboards
setup_grafana_dashboards() {
    log "Setting up Grafana dashboards..."
    
    # Create datasource configuration
    cat > "$MONITORING_DIR/datasources/prometheus.yml" << EOF
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: true
    jsonData:
      httpMethod: POST
      manageAlerts: true
      prometheusType: Prometheus
      prometheusVersion: 2.40.0
      cacheLevel: 'High'
      disableMetricsLookup: false
      incrementalQueryOverlapWindow: 10m
      queryTimeout: 60s
      timeInterval: 15s
EOF

    # Create dashboard provisioning configuration
    cat > "$MONITORING_DIR/dashboards/dashboard.yml" << EOF
apiVersion: 1

providers:
  - name: 'ippan-dashboards'
    orgId: 1
    folder: 'IPPAN'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /etc/grafana/provisioning/dashboards
EOF

    log "Grafana dashboards configured"
}

# Set up Fluentd configuration
setup_fluentd() {
    log "Setting up Fluentd configuration..."
    
    cat > "$MONITORING_DIR/fluentd.conf" << EOF
<source>
  @type forward
  port 24224
  bind 0.0.0.0
</source>

<match ippan.**>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name ippan-logs
  type_name _doc
  include_tag_key true
  tag_key @log_name
  flush_interval 1s
</match>

<match **>
  @type stdout
</match>
EOF

    log "Fluentd configuration created"
}

# Set up monitoring stack
setup_monitoring_stack() {
    log "Setting up monitoring stack..."
    
    cd "$MONITORING_DIR"
    
    # Pull Docker images
    log "Pulling Docker images..."
    docker-compose -f docker-compose-production.yml pull
    
    # Start monitoring stack
    log "Starting monitoring stack..."
    docker-compose -f docker-compose-production.yml up -d
    
    # Wait for services to be ready
    log "Waiting for services to be ready..."
    sleep 30
    
    # Check service health
    check_service_health
    
    cd - > /dev/null
    log "Monitoring stack started"
}

# Check service health
check_service_health() {
    log "Checking service health..."
    
    # Check Prometheus
    if curl -f http://localhost:9090/-/healthy > /dev/null 2>&1; then
        log "Prometheus is healthy"
    else
        warn "Prometheus health check failed"
    fi
    
    # Check AlertManager
    if curl -f http://localhost:9093/-/healthy > /dev/null 2>&1; then
        log "AlertManager is healthy"
    else
        warn "AlertManager health check failed"
    fi
    
    # Check Grafana
    if curl -f http://localhost:3000/api/health > /dev/null 2>&1; then
        log "Grafana is healthy"
    else
        warn "Grafana health check failed"
    fi
    
    # Check Elasticsearch
    if curl -f http://localhost:9200/_cluster/health > /dev/null 2>&1; then
        log "Elasticsearch is healthy"
    else
        warn "Elasticsearch health check failed"
    fi
    
    # Check Kibana
    if curl -f http://localhost:5601/api/status > /dev/null 2>&1; then
        log "Kibana is healthy"
    else
        warn "Kibana health check failed"
    fi
}

# Set up monitoring scripts
setup_monitoring_scripts() {
    log "Setting up monitoring scripts..."
    
    # Create monitoring status script
    cat > "scripts/monitoring-status.sh" << 'EOF'
#!/bin/bash

# IPPAN Monitoring Status Script

echo "=== IPPAN Monitoring Stack Status ==="
echo

# Check Docker containers
echo "Docker Containers:"
docker-compose -f deployments/monitoring/docker-compose-production.yml ps
echo

# Check service endpoints
echo "Service Endpoints:"
echo "Prometheus: http://localhost:9090"
echo "AlertManager: http://localhost:9093"
echo "Grafana: http://localhost:3000"
echo "Elasticsearch: http://localhost:9200"
echo "Kibana: http://localhost:5601"
echo "Jaeger: http://localhost:16686"
echo "Uptime Kuma: http://localhost:3001"
echo

# Check service health
echo "Service Health:"
curl -s http://localhost:9090/-/healthy && echo " - Prometheus: OK" || echo " - Prometheus: FAILED"
curl -s http://localhost:9093/-/healthy && echo " - AlertManager: OK" || echo " - AlertManager: FAILED"
curl -s http://localhost:3000/api/health && echo " - Grafana: OK" || echo " - Grafana: FAILED"
curl -s http://localhost:9200/_cluster/health && echo " - Elasticsearch: OK" || echo " - Elasticsearch: FAILED"
curl -s http://localhost:5601/api/status && echo " - Kibana: OK" || echo " - Kibana: FAILED"
EOF

    chmod +x "scripts/monitoring-status.sh"
    
    # Create monitoring logs script
    cat > "scripts/monitoring-logs.sh" << 'EOF'
#!/bin/bash

# IPPAN Monitoring Logs Script

SERVICE=${1:-"all"}

case $SERVICE in
    "prometheus")
        docker-compose -f deployments/monitoring/docker-compose-production.yml logs -f prometheus
        ;;
    "alertmanager")
        docker-compose -f deployments/monitoring/docker-compose-production.yml logs -f alertmanager
        ;;
    "grafana")
        docker-compose -f deployments/monitoring/docker-compose-production.yml logs -f grafana
        ;;
    "elasticsearch")
        docker-compose -f deployments/monitoring/docker-compose-production.yml logs -f elasticsearch
        ;;
    "kibana")
        docker-compose -f deployments/monitoring/docker-compose-production.yml logs -f kibana
        ;;
    "all")
        docker-compose -f deployments/monitoring/docker-compose-production.yml logs -f
        ;;
    *)
        echo "Usage: $0 [prometheus|alertmanager|grafana|elasticsearch|kibana|all]"
        exit 1
        ;;
esac
EOF

    chmod +x "scripts/monitoring-logs.sh"
    
    log "Monitoring scripts created"
}

# Set up monitoring alerts
setup_monitoring_alerts() {
    log "Setting up monitoring alerts..."
    
    # Create alert test script
    cat > "scripts/test-alerts.sh" << 'EOF'
#!/bin/bash

# IPPAN Alert Testing Script

echo "Testing IPPAN monitoring alerts..."

# Test Prometheus alert rules
echo "Testing Prometheus alert rules..."
curl -s http://localhost:9090/api/v1/rules | jq '.data.groups[].rules[] | select(.state == "firing") | {alertname: .name, state: .state, severity: .labels.severity}'

# Test AlertManager configuration
echo "Testing AlertManager configuration..."
curl -s http://localhost:9093/api/v1/status | jq '.data.config'

# Test Grafana alerting
echo "Testing Grafana alerting..."
curl -s -u admin:secure-grafana-password http://localhost:3000/api/alerting/rules | jq '.data[] | {title: .title, state: .state}'

echo "Alert testing completed"
EOF

    chmod +x "scripts/test-alerts.sh"
    
    log "Monitoring alerts configured"
}

# Set up monitoring backup
setup_monitoring_backup() {
    log "Setting up monitoring backup..."
    
    # Create backup script
    cat > "scripts/backup-monitoring.sh" << 'EOF'
#!/bin/bash

# IPPAN Monitoring Backup Script

BACKUP_DIR="backups/monitoring/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

echo "Backing up monitoring configuration to $BACKUP_DIR..."

# Backup configuration files
cp -r deployments/monitoring/*.yml "$BACKUP_DIR/"
cp -r deployments/monitoring/dashboards "$BACKUP_DIR/"
cp -r deployments/monitoring/datasources "$BACKUP_DIR/"

# Backup Grafana data
docker run --rm -v ippan_grafana_data:/data -v "$(pwd)/$BACKUP_DIR":/backup alpine tar czf /backup/grafana-data.tar.gz -C /data .

# Backup Prometheus data
docker run --rm -v ippan_prometheus_data:/data -v "$(pwd)/$BACKUP_DIR":/backup alpine tar czf /backup/prometheus-data.tar.gz -C /data .

# Backup AlertManager data
docker run --rm -v ippan_alertmanager_data:/data -v "$(pwd)/$BACKUP_DIR":/backup alpine tar czf /backup/alertmanager-data.tar.gz -C /data .

echo "Monitoring backup completed: $BACKUP_DIR"
EOF

    chmod +x "scripts/backup-monitoring.sh"
    
    log "Monitoring backup configured"
}

# Main setup function
main() {
    log "Starting IPPAN Production Monitoring Setup..."
    
    # Check prerequisites
    check_prerequisites
    
    # Create backup directory
    create_backup_dir
    
    # Backup existing configuration
    backup_existing_config
    
    # Generate configuration files
    generate_config_files
    
    # Create necessary directories
    create_directories
    
    # Set up Grafana dashboards
    setup_grafana_dashboards
    
    # Set up Fluentd configuration
    setup_fluentd
    
    # Set up monitoring stack
    setup_monitoring_stack
    
    # Set up monitoring scripts
    setup_monitoring_scripts
    
    # Set up monitoring alerts
    setup_monitoring_alerts
    
    # Set up monitoring backup
    setup_monitoring_backup
    
    log "IPPAN Production Monitoring Setup completed successfully!"
    
    echo
    echo "=== Monitoring Stack Information ==="
    echo "Prometheus: http://localhost:9090"
    echo "AlertManager: http://localhost:9093"
    echo "Grafana: http://localhost:3000 (admin/secure-grafana-password)"
    echo "Elasticsearch: http://localhost:9200"
    echo "Kibana: http://localhost:5601"
    echo "Jaeger: http://localhost:16686"
    echo "Uptime Kuma: http://localhost:3001"
    echo
    echo "=== Useful Commands ==="
    echo "Check status: ./scripts/monitoring-status.sh"
    echo "View logs: ./scripts/monitoring-logs.sh [service]"
    echo "Test alerts: ./scripts/test-alerts.sh"
    echo "Backup: ./scripts/backup-monitoring.sh"
    echo
    echo "=== Next Steps ==="
    echo "1. Access Grafana and import dashboards"
    echo "2. Configure alert notifications"
    echo "3. Set up log aggregation"
    echo "4. Configure distributed tracing"
    echo "5. Set up uptime monitoring"
}

# Run main function
main "$@"
