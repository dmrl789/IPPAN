#!/bin/bash

# Quick IPPAN Hetzner Deployment Script
# This script provides a fast deployment option for Hetzner servers

set -e

echo "🚀 IPPAN Quick Hetzner Deployment"
echo "================================="
echo ""

# Get basic information
read -p "Enter your server IP: " SERVER_IP
read -p "Enter domain name (optional): " DOMAIN
read -p "Enter Grafana admin password [admin123]: " ADMIN_PASSWORD
ADMIN_PASSWORD=${ADMIN_PASSWORD:-admin123}

echo ""
echo "📋 Deployment Summary:"
echo "  - Server IP: $SERVER_IP"
echo "  - Domain: ${DOMAIN:-'Not provided'}"
echo "  - Grafana Password: $ADMIN_PASSWORD"
echo ""

read -p "Continue with deployment? (y/N): " CONFIRM
if [[ ! $CONFIRM =~ ^[Yy]$ ]]; then
    echo "Deployment cancelled."
    exit 0
fi

echo ""
echo "🔧 Starting deployment..."

# Update system
echo "Updating system..."
sudo apt update && sudo apt upgrade -y

# Install dependencies
echo "Installing dependencies..."
sudo apt install -y curl wget git vim htop ufw fail2ban certbot nginx apache2-utils jq

# Install Docker
echo "Installing Docker..."
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh
sudo usermod -aG docker $USER

# Install Docker Compose
echo "Installing Docker Compose..."
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Configure firewall
echo "Configuring firewall..."
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

# Create directories
echo "Creating directories..."
sudo mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl}
sudo chown -R $USER:$USER /opt/ippan
sudo chmod -R 755 /opt/ippan

# Generate SSL certificates if domain provided
if [ -n "$DOMAIN" ]; then
    echo "Generating SSL certificates for $DOMAIN..."
    read -p "Enter email for SSL certificates: " EMAIL
    sudo certbot certonly --standalone -d "$DOMAIN" --email "$EMAIL" --agree-tos --non-interactive
    sudo cp "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" /opt/ippan/mainnet/ssl/
    sudo cp "/etc/letsencrypt/live/$DOMAIN/privkey.pem" /opt/ippan/mainnet/ssl/
    sudo chown -R $USER:$USER /opt/ippan/mainnet/ssl/
fi

# Create environment file
echo "Creating environment configuration..."
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

# Domain
DOMAIN=$DOMAIN
EOF

# Configure Nginx if domain provided
if [ -n "$DOMAIN" ]; then
    echo "Configuring Nginx..."
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

    location / {
        proxy_pass http://localhost:3000;
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
fi

# Deploy IPPAN
echo "Deploying IPPAN services..."
docker build -f Dockerfile.production -t ippan/ippan:latest .
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml --env-file .env.mainnet up -d

# Wait for services
echo "Waiting for services to be ready..."
sleep 60

# Verify deployment
echo "Verifying deployment..."
if curl -f http://localhost:3000/api/v1/status > /dev/null 2>&1; then
    echo "✅ IPPAN node is healthy"
else
    echo "❌ IPPAN node health check failed"
    exit 1
fi

if curl -f http://localhost:9090/-/healthy > /dev/null 2>&1; then
    echo "✅ Prometheus is healthy"
else
    echo "❌ Prometheus health check failed"
    exit 1
fi

if curl -f http://localhost:3001/api/health > /dev/null 2>&1; then
    echo "✅ Grafana is healthy"
else
    echo "❌ Grafana health check failed"
    exit 1
fi

# Create maintenance scripts
echo "Creating maintenance scripts..."
cat > ~/update-ippan.sh << 'EOF'
#!/bin/bash
set -e
echo "Updating IPPAN..."
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml down
cp -r /opt/ippan/mainnet/data /opt/ippan/mainnet/data.backup.$(date +%Y%m%d_%H%M%S)
git pull origin main
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml build
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml --env-file .env.mainnet up -d
sleep 30
curl -f http://localhost:3000/api/v1/status
echo "Update completed successfully!"
EOF

cat > ~/backup-ippan.sh << 'EOF'
#!/bin/bash
set -e
BACKUP_DIR="/opt/ippan/mainnet/backups"
DATE=$(date +%Y%m%d_%H%M%S)
echo "Creating backup..."
mkdir -p "$BACKUP_DIR/$DATE"
cp -r /opt/ippan/mainnet/data "$BACKUP_DIR/$DATE/"
cp -r /opt/ippan/mainnet/keys "$BACKUP_DIR/$DATE/"
cp .env.mainnet "$BACKUP_DIR/$DATE/"
tar -czf "$BACKUP_DIR/ippan_backup_$DATE.tar.gz" -C "$BACKUP_DIR" "$DATE"
rm -rf "$BACKUP_DIR/$DATE"
find "$BACKUP_DIR" -name "ippan_backup_*.tar.gz" -mtime +7 -delete
echo "Backup completed: ippan_backup_$DATE.tar.gz"
EOF

chmod +x ~/update-ippan.sh
chmod +x ~/backup-ippan.sh

# Add backup to crontab
(crontab -l 2>/dev/null; echo "0 2 * * * $HOME/backup-ippan.sh") | crontab -

echo ""
echo "🎉 IPPAN deployment completed successfully! 🚀"
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
echo ""
echo "🚨 Emergency Procedures:"
echo "  - Emergency stop: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml down"
echo "  - Emergency restart: docker-compose -f deployments/mainnet/docker-compose.mainnet.yml restart"
echo ""
echo "🌟 IPPAN is now running on your Hetzner server!"
