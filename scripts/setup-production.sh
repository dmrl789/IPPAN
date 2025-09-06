#!/bin/bash

# IPPAN Production Setup Script
# This script sets up the complete production environment for IPPAN

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
IPPAN_VERSION="1.0.0"
DOMAIN_NAME="${DOMAIN_NAME:-ippan.network}"
EMAIL="${EMAIL:-admin@ippan.network}"
ENVIRONMENT="${ENVIRONMENT:-production}"

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        error "This script should not be run as root for security reasons"
    fi
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check required commands
    local required_commands=("docker" "docker-compose" "curl" "openssl" "certbot")
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            error "$cmd is not installed. Please install it first."
        fi
    done
    
    # Check Docker is running
    if ! docker info &> /dev/null; then
        error "Docker is not running. Please start Docker first."
    fi
    
    # Check available disk space (minimum 10GB)
    local available_space=$(df / | awk 'NR==2 {print $4}')
    local min_space=$((10 * 1024 * 1024)) # 10GB in KB
    
    if [[ $available_space -lt $min_space ]]; then
        error "Insufficient disk space. At least 10GB free space required."
    fi
    
    success "Prerequisites check completed"
}

# Setup SSL certificates
setup_ssl() {
    log "Setting up SSL certificates..."
    
    # Create SSL directory
    sudo mkdir -p /etc/nginx/ssl
    
    if [[ "$ENVIRONMENT" == "production" ]]; then
        # Use Let's Encrypt for production
        log "Obtaining Let's Encrypt certificate for $DOMAIN_NAME"
        
        # Stop nginx if running
        sudo systemctl stop nginx 2>/dev/null || true
        
        # Obtain certificate
        sudo certbot certonly --standalone \
            --non-interactive \
            --agree-tos \
            --email "$EMAIL" \
            -d "$DOMAIN_NAME" \
            -d "www.$DOMAIN_NAME" \
            -d "api.$DOMAIN_NAME"
        
        # Create symlinks to nginx ssl directory
        sudo ln -sf "/etc/letsencrypt/live/$DOMAIN_NAME/fullchain.pem" "/etc/nginx/ssl/$DOMAIN_NAME.crt"
        sudo ln -sf "/etc/letsencrypt/live/$DOMAIN_NAME/privkey.pem" "/etc/nginx/ssl/$DOMAIN_NAME.key"
        
        # Setup certificate renewal
        sudo crontab -l 2>/dev/null | grep -v certbot | sudo crontab -
        echo "0 12 * * * /usr/bin/certbot renew --quiet && systemctl reload nginx" | sudo crontab -
        
    else
        # Generate self-signed certificate for development
        log "Generating self-signed certificate for development"
        
        sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
            -keyout "/etc/nginx/ssl/$DOMAIN_NAME.key" \
            -out "/etc/nginx/ssl/$DOMAIN_NAME.crt" \
            -subj "/C=US/ST=State/L=City/O=IPPAN/CN=$DOMAIN_NAME"
    fi
    
    # Set proper permissions
    sudo chmod 600 "/etc/nginx/ssl/$DOMAIN_NAME.key"
    sudo chmod 644 "/etc/nginx/ssl/$DOMAIN_NAME.crt"
    
    success "SSL certificates configured"
}

# Setup environment files
setup_environment() {
    log "Setting up environment configuration..."
    
    # Create production environment file
    cat > .env.production << EOF
# IPPAN Production Environment Variables
IPPAN_VERSION=$IPPAN_VERSION
DEPLOYMENT_ENV=$ENVIRONMENT
DOMAIN_NAME=$DOMAIN_NAME

# Network Configuration
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000

# Storage Configuration
IPPAN_STORAGE_DIR=/data
IPPAN_KEYS_DIR=/keys
IPPAN_LOG_DIR=/logs

# Security Configuration
BACKUP_ENCRYPTION_KEY=$(openssl rand -hex 32)
JWT_SECRET=$(openssl rand -hex 32)
DATABASE_PASSWORD=$(openssl rand -hex 16)

# SSL Configuration
SSL_CERT_PATH=/etc/nginx/ssl/$DOMAIN_NAME.crt
SSL_KEY_PATH=/etc/nginx/ssl/$DOMAIN_NAME.key

# External Services
SLACK_WEBHOOK_URL=${SLACK_WEBHOOK_URL:-}
EMAIL_SMTP_HOST=${EMAIL_SMTP_HOST:-}
EMAIL_SMTP_PORT=${EMAIL_SMTP_PORT:-587}
EMAIL_USERNAME=${EMAIL_USERNAME:-}
EMAIL_PASSWORD=${EMAIL_PASSWORD:-}

# Monitoring
PROMETHEUS_RETENTION_TIME=15d
GRAFANA_ADMIN_PASSWORD=$(openssl rand -hex 12)
ALERTMANAGER_WEBHOOK_URL=${ALERTMANAGER_WEBHOOK_URL:-}
EOF

    # Create frontend environment files
    cat > apps/unified-ui/.env.production << EOF
VITE_API_URL=https://api.$DOMAIN_NAME
VITE_API_VERSION=v1
VITE_NETWORK_NAME=IPPAN
VITE_CHAIN_ID=1
VITE_ENABLE_ANALYTICS=true
VITE_ENABLE_ERROR_REPORTING=true
VITE_WEBSOCKET_URL=wss://api.$DOMAIN_NAME/ws
VITE_EXPLORER_URL=https://explorer.$DOMAIN_NAME
EOF

    # Set secure permissions
    chmod 600 .env.production
    chmod 600 apps/unified-ui/.env.production
    
    success "Environment configuration created"
}

# Setup monitoring stack
setup_monitoring() {
    log "Setting up monitoring stack..."
    
    # Create monitoring directories
    sudo mkdir -p /opt/ippan/monitoring/{prometheus,grafana,alertmanager}
    sudo mkdir -p /opt/ippan/monitoring/grafana/{dashboards,datasources}
    
    # Copy monitoring configurations
    sudo cp deployments/monitoring/prometheus.yml /opt/ippan/monitoring/prometheus/
    sudo cp deployments/monitoring/alerts.yml /opt/ippan/monitoring/alertmanager/
    
    # Create Grafana datasource configuration
    sudo tee /opt/ippan/monitoring/grafana/datasources/prometheus.yml > /dev/null << EOF
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
EOF

    # Create comprehensive IPPAN dashboard
    sudo tee /opt/ippan/monitoring/grafana/dashboards/ippan-comprehensive.json > /dev/null << 'EOF'
{
  "dashboard": {
    "id": null,
    "title": "IPPAN Node Comprehensive Dashboard",
    "tags": ["ippan", "blockchain", "monitoring"],
    "timezone": "browser",
    "panels": [
      {
        "id": 1,
        "title": "Node Status",
        "type": "stat",
        "targets": [
          {
            "expr": "ippan_node_status",
            "legendFormat": "Status"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "color": {
              "mode": "thresholds"
            },
            "thresholds": {
              "steps": [
                {"color": "red", "value": 0},
                {"color": "green", "value": 1}
              ]
            }
          }
        },
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
      },
      {
        "id": 2,
        "title": "Connected Peers",
        "type": "graph",
        "targets": [
          {
            "expr": "ippan_network_connected_peers",
            "legendFormat": "Peers"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
      },
      {
        "id": 3,
        "title": "Storage Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "(ippan_storage_used_bytes / ippan_storage_total_bytes) * 100",
            "legendFormat": "Usage %"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8}
      },
      {
        "id": 4,
        "title": "Transaction Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(ippan_transactions_total[5m])",
            "legendFormat": "TPS"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8}
      },
      {
        "id": 5,
        "title": "Consensus Round",
        "type": "stat",
        "targets": [
          {
            "expr": "ippan_consensus_round",
            "legendFormat": "Round"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 16}
      },
      {
        "id": 6,
        "title": "API Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(ippan_api_request_duration_seconds_bucket[5m])) * 1000",
            "legendFormat": "95th percentile"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 16}
      }
    ],
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "refresh": "5s"
  }
}
EOF

    # Set proper ownership
    sudo chown -R 472:472 /opt/ippan/monitoring/grafana  # Grafana user ID
    sudo chown -R 65534:65534 /opt/ippan/monitoring/prometheus  # Nobody user ID
    
    success "Monitoring stack configured"
}

# Setup database
setup_database() {
    log "Setting up database..."
    
    # Create database directories
    sudo mkdir -p /opt/ippan/data/postgresql
    sudo chown -R 999:999 /opt/ippan/data/postgresql  # PostgreSQL user ID
    
    # Create database initialization script
    sudo mkdir -p /opt/ippan/scripts/db
    sudo tee /opt/ippan/scripts/db/init.sql > /dev/null << 'EOF'
-- IPPAN Database Initialization
CREATE DATABASE ippan_production;
CREATE USER ippan_api WITH ENCRYPTED PASSWORD 'secure_password_change_me';
GRANT ALL PRIVILEGES ON DATABASE ippan_production TO ippan_api;

-- Create extensions
\c ippan_production;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";

-- Create basic tables (expand as needed)
CREATE TABLE IF NOT EXISTS nodes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    node_id VARCHAR(64) UNIQUE NOT NULL,
    address VARCHAR(255) NOT NULL,
    status VARCHAR(20) DEFAULT 'active',
    last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tx_hash VARCHAR(64) UNIQUE NOT NULL,
    from_address VARCHAR(64) NOT NULL,
    to_address VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    fee BIGINT NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    block_height BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS domains (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) UNIQUE NOT NULL,
    owner VARCHAR(64) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    price BIGINT NOT NULL,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_nodes_node_id ON nodes(node_id);
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);
CREATE INDEX IF NOT EXISTS idx_transactions_hash ON transactions(tx_hash);
CREATE INDEX IF NOT EXISTS idx_transactions_addresses ON transactions(from_address, to_address);
CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status);
CREATE INDEX IF NOT EXISTS idx_domains_name ON domains(name);
CREATE INDEX IF NOT EXISTS idx_domains_owner ON domains(owner);
EOF

    success "Database setup completed"
}

# Build and push Docker images
build_images() {
    log "Building Docker images..."
    
    # Build main IPPAN image
    docker build -t ippan/ippan:$IPPAN_VERSION -t ippan/ippan:latest .
    
    # Build frontend images
    docker build -t ippan/unified-ui:$IPPAN_VERSION -t ippan/unified-ui:latest ./apps/unified-ui
    docker build -t ippan/wallet:$IPPAN_VERSION -t ippan/wallet:latest ./apps/wallet
    docker build -t ippan/neuro-ui:$IPPAN_VERSION -t ippan/neuro-ui:latest ./neuro-ui
    
    success "Docker images built successfully"
}

# Setup backup system
setup_backup() {
    log "Setting up backup system..."
    
    # Create backup directories
    sudo mkdir -p /opt/ippan/backups/{daily,weekly,monthly}
    
    # Create backup script
    sudo tee /opt/ippan/scripts/backup.sh > /dev/null << 'EOF'
#!/bin/bash

# IPPAN Backup Script
set -euo pipefail

BACKUP_TYPE=${1:-daily}
BACKUP_DIR="/opt/ippan/backups/$BACKUP_TYPE"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="ippan_backup_$TIMESTAMP"

# Create backup directory
mkdir -p "$BACKUP_DIR/$BACKUP_NAME"

# Backup IPPAN data
echo "Backing up IPPAN data..."
docker exec ippan-node tar czf - /data > "$BACKUP_DIR/$BACKUP_NAME/data.tar.gz"

# Backup keys
echo "Backing up keys..."
docker exec ippan-node tar czf - /keys > "$BACKUP_DIR/$BACKUP_NAME/keys.tar.gz"

# Backup database
echo "Backing up database..."
docker exec ippan-db pg_dump -U ippan ippan_production | gzip > "$BACKUP_DIR/$BACKUP_NAME/database.sql.gz"

# Backup configuration
echo "Backing up configuration..."
tar czf "$BACKUP_DIR/$BACKUP_NAME/config.tar.gz" /opt/ippan/config/ 2>/dev/null || true

# Create backup manifest
cat > "$BACKUP_DIR/$BACKUP_NAME/manifest.json" << EOL
{
  "backup_type": "$BACKUP_TYPE",
  "timestamp": "$TIMESTAMP",
  "version": "$(docker exec ippan-node ippan --version 2>/dev/null || echo 'unknown')",
  "files": [
    "data.tar.gz",
    "keys.tar.gz",
    "database.sql.gz",
    "config.tar.gz"
  ]
}
EOL

# Encrypt backup (if encryption key is available)
if [[ -n "${BACKUP_ENCRYPTION_KEY:-}" ]]; then
    echo "Encrypting backup..."
    tar czf - -C "$BACKUP_DIR" "$BACKUP_NAME" | \
        openssl enc -aes-256-cbc -salt -k "$BACKUP_ENCRYPTION_KEY" > \
        "$BACKUP_DIR/${BACKUP_NAME}.encrypted"
    rm -rf "$BACKUP_DIR/$BACKUP_NAME"
fi

# Cleanup old backups (keep last 7 daily, 4 weekly, 12 monthly)
case $BACKUP_TYPE in
    daily) find "$BACKUP_DIR" -type f -name "*.tar.gz" -o -name "*.encrypted" | sort | head -n -7 | xargs rm -f ;;
    weekly) find "$BACKUP_DIR" -type f -name "*.tar.gz" -o -name "*.encrypted" | sort | head -n -4 | xargs rm -f ;;
    monthly) find "$BACKUP_DIR" -type f -name "*.tar.gz" -o -name "*.encrypted" | sort | head -n -12 | xargs rm -f ;;
esac

echo "Backup completed: $BACKUP_NAME"
EOF

    sudo chmod +x /opt/ippan/scripts/backup.sh
    
    # Setup cron jobs for automated backups
    (sudo crontab -l 2>/dev/null | grep -v "ippan backup" || true; echo "0 2 * * * /opt/ippan/scripts/backup.sh daily") | sudo crontab -
    (sudo crontab -l 2>/dev/null | grep -v "ippan backup weekly" || true; echo "0 3 * * 0 /opt/ippan/scripts/backup.sh weekly") | sudo crontab -
    (sudo crontab -l 2>/dev/null | grep -v "ippan backup monthly" || true; echo "0 4 1 * * /opt/ippan/scripts/backup.sh monthly") | sudo crontab -
    
    success "Backup system configured"
}

# Setup log rotation
setup_logging() {
    log "Setting up log rotation..."
    
    sudo tee /etc/logrotate.d/ippan > /dev/null << 'EOF'
/opt/ippan/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 root root
    postrotate
        docker kill -s USR1 ippan-node 2>/dev/null || true
    endscript
}
EOF

    success "Log rotation configured"
}

# Deploy the complete stack
deploy_stack() {
    log "Deploying IPPAN production stack..."
    
    # Create necessary directories
    sudo mkdir -p /opt/ippan/{config,data,logs,keys}
    
    # Copy configuration files
    sudo cp -r config/* /opt/ippan/config/
    sudo cp -r deployments/* /opt/ippan/
    
    # Start the stack
    docker-compose -f /opt/ippan/production/docker-compose.yml --env-file .env.production up -d
    
    success "IPPAN stack deployed"
}

# Verify deployment
verify_deployment() {
    log "Verifying deployment..."
    
    local max_attempts=60
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        log "Verification attempt $attempt/$max_attempts"
        
        # Check API health
        if curl -f -s "https://api.$DOMAIN_NAME/api/v1/health" > /dev/null 2>&1; then
            success "API is responding"
            
            # Check node status
            local status=$(curl -s "https://api.$DOMAIN_NAME/api/v1/status" | jq -r '.status' 2>/dev/null || echo "unknown")
            if [[ "$status" == "running" ]]; then
                success "IPPAN node is running"
                break
            else
                warning "Node status: $status"
            fi
        else
            warning "API not responding yet"
        fi
        
        sleep 10
        ((attempt++))
    done
    
    if [[ $attempt -gt $max_attempts ]]; then
        error "Deployment verification failed"
    fi
    
    success "Deployment verified successfully"
}

# Main setup function
main() {
    log "Starting IPPAN production setup..."
    
    # Run setup steps
    check_root
    check_prerequisites
    setup_ssl
    setup_environment
    setup_monitoring
    setup_database
    build_images
    setup_backup
    setup_logging
    deploy_stack
    verify_deployment
    
    success "IPPAN production setup completed successfully!"
    
    # Display setup summary
    echo
    echo "=== IPPAN Production Setup Summary ==="
    echo "Domain: $DOMAIN_NAME"
    echo "Version: $IPPAN_VERSION"
    echo "Environment: $ENVIRONMENT"
    echo
    echo "Services:"
    echo "  - Main API: https://api.$DOMAIN_NAME"
    echo "  - Unified UI: https://$DOMAIN_NAME"
    echo "  - Wallet UI: https://wallet.$DOMAIN_NAME"
    echo "  - Neuro UI: https://neuro.$DOMAIN_NAME"
    echo "  - Monitoring: https://monitoring.$DOMAIN_NAME"
    echo "  - Grafana: https://grafana.$DOMAIN_NAME"
    echo
    echo "Management commands:"
    echo "  - View logs: docker logs ippan-node"
    echo "  - Check status: curl https://api.$DOMAIN_NAME/api/v1/status"
    echo "  - Manual backup: sudo /opt/ippan/scripts/backup.sh"
    echo "  - Stop services: docker-compose -f /opt/ippan/production/docker-compose.yml down"
    echo
    echo "Configuration files are stored in /opt/ippan/"
    echo "Backups are stored in /opt/ippan/backups/"
    echo "SSL certificates will auto-renew via cron"
    echo
}

# Run main function
main "$@"
