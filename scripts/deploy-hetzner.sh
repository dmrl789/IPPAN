#!/bin/bash

# IPPAN Hetzner Server Deployment Script
# This script automates the deployment of IPPAN on Hetzner Cloud servers

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Configuration
SERVER_IP=""
DOMAIN=""
EMAIL=""
ADMIN_PASSWORD=""

# Function to get user input
get_input() {
    local prompt="$1"
    local var_name="$2"
    local default="$3"
    
    if [ -n "$default" ]; then
        read -p "$prompt [$default]: " input
        eval "$var_name=\${input:-$default}"
    else
        read -p "$prompt: " input
        eval "$var_name=\"$input\""
    fi
}

# Function to check if running as root
check_root() {
    if [ "$EUID" -eq 0 ]; then
        log_error "This script should not be run as root. Please run as a regular user with sudo privileges."
        exit 1
    fi
}

# Function to install dependencies
install_dependencies() {
    log_info "Installing system dependencies..."
    
    sudo apt update && sudo apt upgrade -y
    sudo apt install -y curl wget git vim htop ufw fail2ban certbot nginx apache2-utils jq
    
    log_success "Dependencies installed successfully"
}

# Function to install Docker
install_docker() {
    log_info "Installing Docker and Docker Compose..."
    
    # Install Docker
    curl -fsSL https://get.docker.com -o get-docker.sh
    sh get-docker.sh
    sudo usermod -aG docker $USER
    
    # Install Docker Compose
    sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
    sudo chmod +x /usr/local/bin/docker-compose
    
    # Verify installation
    docker --version
    docker-compose --version
    
    log_success "Docker installed successfully"
}

# Function to configure firewall
configure_firewall() {
    log_info "Configuring firewall..."
    
    sudo ufw default deny incoming
    sudo ufw default allow outgoing
    sudo ufw allow ssh
    sudo ufw allow 80/tcp
    sudo ufw allow 443/tcp
    sudo ufw allow 3000/tcp
    sudo ufw allow 8080/tcp
    sudo ufw allow 9090/tcp
    sudo ufw allow 3001/tcp
    sudo ufw --force enable
    
    log_success "Firewall configured successfully"
}

# Function to create directories
create_directories() {
    log_info "Creating IPPAN directories..."
    
    sudo mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl}
    sudo chown -R $USER:$USER /opt/ippan
    sudo chmod -R 755 /opt/ippan
    
    log_success "Directories created successfully"
}

# Function to generate SSL certificates
generate_ssl_certificates() {
    if [ -n "$DOMAIN" ] && [ -n "$EMAIL" ]; then
        log_info "Generating SSL certificates for $DOMAIN..."
        
        sudo certbot certonly --standalone -d "$DOMAIN" --email "$EMAIL" --agree-tos --non-interactive
        
        # Copy certificates
        sudo cp "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" /opt/ippan/mainnet/ssl/
        sudo cp "/etc/letsencrypt/live/$DOMAIN/privkey.pem" /opt/ippan/mainnet/ssl/
        sudo chown -R $USER:$USER /opt/ippan/mainnet/ssl/
        
        log_success "SSL certificates generated successfully"
    else
        log_warning "Skipping SSL certificate generation (domain or email not provided)"
    fi
}

# Function to create environment file
create_environment_file() {
    log_info "Creating environment configuration..."
    
    # Generate secure passwords
    POSTGRES_PASSWORD=$(openssl rand -base64 32)
    REDIS_PASSWORD=$(openssl rand -base64 32)
    JWT_SECRET=$(openssl rand -base64 64)
    BACKUP_ENCRYPTION_KEY=$(openssl rand -base64 32)
    
    cat > .env.mainnet << EOF
# IPPAN Mainnet Environment Configuration
POSTGRES_DB=ippan_mainnet
POSTGRES_USER=ippan
POSTGRES_PASSWORD=$POSTGRES_PASSWORD
REDIS_PASSWORD=$REDIS_PASSWORD
JWT_SECRET=$JWT_SECRET
GRAFANA_ADMIN_PASSWORD=$ADMIN_PASSWORD
GRAFANA_SECRET_KEY=$(openssl rand -base64 32)
BACKUP_ENCRYPTION_KEY=$BACKUP_ENCRYPTION_KEY

# Network
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000
IPPAN_P2P_PORT=8080

# Production flags
NODE_ENV=production
IPPAN_ENVIRONMENT=mainnet
RUST_LOG=info
LOG_LEVEL=info
LOG_FORMAT=json

# Security
IPPAN_ENABLE_TLS=true
IPPAN_ENABLE_MUTUAL_AUTH=true
IPPAN_MAX_CONNECTIONS=10000
IPPAN_THREAD_POOL_SIZE=32
IPPAN_CACHE_SIZE=4294967296
IPPAN_MEMORY_POOL_SIZE=2147483648

# Monitoring
PROMETHEUS_RETENTION=30d
GRAFANA_ADMIN_USER=admin
ALERTMANAGER_CONFIG_FILE=/etc/alertmanager/alertmanager.yml

# Domain (if provided)
DOMAIN=$DOMAIN
EOF
    
    log_success "Environment file created successfully"
}

# Function to configure Nginx
configure_nginx() {
    if [ -n "$DOMAIN" ]; then
        log_info "Configuring Nginx for domain $DOMAIN..."
        
        sudo tee /etc/nginx/sites-available/ippan > /dev/null << EOF
server {
    listen 80;
    server_name $DOMAIN;
    return 301 https://\$server_name\$request_uri;
}

server {
    listen 443 ssl http2;
    server_name $DOMAIN;

    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    location /api/ {
        proxy_pass http://localhost:3000/api/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF
        
        sudo ln -sf /etc/nginx/sites-available/ippan /etc/nginx/sites-enabled/
        sudo rm -f /etc/nginx/sites-enabled/default
        sudo nginx -t
        sudo systemctl restart nginx
        
        log_success "Nginx configured successfully"
    else
        log_warning "Skipping Nginx configuration (domain not provided)"
    fi
}

# Function to deploy IPPAN
deploy_ippan() {
    log_info "Deploying IPPAN services..."
    
    # Build Docker images
    docker build -f Dockerfile.production -t ippan/ippan:latest .
    
    # Deploy services
    docker-compose -f deployments/mainnet/docker-compose.mainnet.yml --env-file .env.mainnet up -d
    
    log_success "IPPAN services deployed successfully"
}

# Function to wait for services
wait_for_services() {
    log_info "Waiting for services to be ready..."
    
    # Wait for IPPAN node
    log_info "Waiting for IPPAN node..."
    timeout 300 bash -c 'until curl -f http://localhost:3000/api/v1/status; do sleep 10; done'
    
    # Wait for Prometheus
    log_info "Waiting for Prometheus..."
    timeout 300 bash -c 'until curl -f http://localhost:9090/-/healthy; do sleep 5; done'
    
    # Wait for Grafana
    log_info "Waiting for Grafana..."
    timeout 300 bash -c 'until curl -f http://localhost:3001/api/health; do sleep 5; done'
    
    log_success "All services are ready"
}

# Function to verify deployment
verify_deployment() {
    log_info "Verifying deployment..."
    
    # Check IPPAN node
    if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
        log_success "IPPAN node is healthy"
    else
        log_error "IPPAN node health check failed"
        return 1
    fi
    
    # Check Prometheus
    if curl -f http://localhost:9090/-/healthy > /dev/null 2>&1; then
        log_success "Prometheus is healthy"
    else
        log_error "Prometheus health check failed"
        return 1
    fi
    
    # Check Grafana
    if curl -f http://localhost:3001/api/health > /dev/null 2>&1; then
        log_success "Grafana is healthy"
    else
        log_error "Grafana health check failed"
        return 1
    fi
    
    log_success "Deployment verification completed successfully"
}

# Function to create maintenance scripts
create_maintenance_scripts() {
    log_info "Creating maintenance scripts..."
    
    # Create update script
    cat > ~/update-ippan.sh << 'EOF'
#!/bin/bash
set -e

echo "Updating IPPAN..."

# Backup current deployment
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml down
cp -r /opt/ippan/mainnet/data /opt/ippan/mainnet/data.backup.$(date +%Y%m%d_%H%M%S)

# Pull latest changes
git pull origin main

# Rebuild and restart
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml build
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml --env-file .env.mainnet up -d

# Verify deployment
sleep 30
curl -f http://localhost:3000/api/v1/status

echo "Update completed successfully!"
EOF
    
    # Create backup script
    cat > ~/backup-ippan.sh << 'EOF'
#!/bin/bash
set -e

BACKUP_DIR="/opt/ippan/mainnet/backups"
DATE=$(date +%Y%m%d_%H%M%S)

echo "Creating backup..."

# Create backup directory
mkdir -p "$BACKUP_DIR/$DATE"

# Backup data
cp -r /opt/ippan/mainnet/data "$BACKUP_DIR/$DATE/"
cp -r /opt/ippan/mainnet/keys "$BACKUP_DIR/$DATE/"

# Backup configuration
cp .env.mainnet "$BACKUP_DIR/$DATE/"

# Compress backup
tar -czf "$BACKUP_DIR/ippan_backup_$DATE.tar.gz" -C "$BACKUP_DIR" "$DATE"
rm -rf "$BACKUP_DIR/$DATE"

# Keep only last 7 days of backups
find "$BACKUP_DIR" -name "ippan_backup_*.tar.gz" -mtime +7 -delete

echo "Backup completed: ippan_backup_$DATE.tar.gz"
EOF
    
    chmod +x ~/update-ippan.sh
    chmod +x ~/backup-ippan.sh
    
    # Add backup to crontab
    (crontab -l 2>/dev/null; echo "0 2 * * * $HOME/backup-ippan.sh") | crontab -
    
    log_success "Maintenance scripts created successfully"
}

# Function to show deployment information
show_deployment_info() {
    log_success "IPPAN deployment completed successfully! 🚀"
    echo ""
    echo "🎉 IPPAN Mainnet is now LIVE on Hetzner!"
    echo ""
    echo "📊 Service URLs:"
    if [ -n "$DOMAIN" ]; then
        echo "  - IPPAN API: https://$DOMAIN"
        echo "  - Prometheus: https://$DOMAIN:9090"
        echo "  - Grafana: https://$DOMAIN:3001"
    else
        echo "  - IPPAN API: http://$SERVER_IP:3000"
        echo "  - Prometheus: http://$SERVER_IP:9090"
        echo "  - Grafana: http://$SERVER_IP:3001"
    fi
    echo ""
    echo "📋 Useful Commands:"
    echo "  - View logs: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml logs -f"
    echo "  - Stop services: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml down"
    echo "  - Restart services: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml restart"
    echo "  - Update IPPAN: ~/update-ippan.sh"
    echo "  - Create backup: ~/backup-ippan.sh"
    echo ""
    echo "🔐 Configuration:"
    echo "  - Environment file: .env.mainnet"
    echo "  - Data directory: /opt/ippan/mainnet/data"
    echo "  - Logs directory: /opt/ippan/mainnet/logs"
    echo "  - Backup directory: /opt/ippan/mainnet/backups"
    echo ""
    echo "📊 Monitoring:"
    echo "  - Grafana admin password: $ADMIN_PASSWORD"
    echo "  - Prometheus metrics: http://localhost:9090/metrics"
    echo "  - System monitoring: htop, docker stats"
    echo ""
    echo "🚨 Emergency Procedures:"
    echo "  - Emergency stop: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml down"
    echo "  - Emergency restart: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml restart"
    echo "  - View logs: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml logs"
    echo ""
}

# Main deployment function
main() {
    log_info "Starting IPPAN Hetzner deployment..."
    
    # Check if running as root
    check_root
    
    # Get user input
    get_input "Enter server IP address" "SERVER_IP"
    get_input "Enter domain name (optional)" "DOMAIN" ""
    get_input "Enter email for SSL certificates (optional)" "EMAIL" ""
    get_input "Enter Grafana admin password" "ADMIN_PASSWORD" "admin123"
    
    # Install dependencies
    install_dependencies
    install_docker
    
    # Configure system
    configure_firewall
    create_directories
    
    # Generate SSL certificates if domain provided
    if [ -n "$DOMAIN" ] && [ -n "$EMAIL" ]; then
        generate_ssl_certificates
    fi
    
    # Create configuration
    create_environment_file
    configure_nginx
    
    # Deploy IPPAN
    deploy_ippan
    wait_for_services
    verify_deployment
    
    # Create maintenance scripts
    create_maintenance_scripts
    
    # Show deployment information
    show_deployment_info
}

# Run main function
main "$@"
