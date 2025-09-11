# 🚀 IPPAN Hetzner Server Deployment Guide

This guide provides step-by-step instructions for deploying IPPAN blockchain on Hetzner Cloud servers for production use.

## 📋 Prerequisites

### Required Accounts
- **Hetzner Cloud Account**: [Sign up at Hetzner Cloud](https://www.hetzner.com/cloud)
- **Domain Name**: For SSL certificates and public access
- **SSH Key**: For secure server access

### Required Tools
- **SSH Client**: PuTTY (Windows) or Terminal (Mac/Linux)
- **SCP/SFTP Client**: For file transfers
- **Domain DNS Access**: To configure DNS records

## 🖥️ Server Requirements

### Minimum Configuration (Development/Testing)
- **CPU**: 4 vCPUs
- **RAM**: 8 GB
- **Storage**: 100 GB SSD
- **Network**: 20 TB traffic
- **Cost**: ~€20/month

### Recommended Configuration (Production)
- **CPU**: 8 vCPUs
- **RAM**: 32 GB
- **Storage**: 500 GB SSD
- **Network**: 20 TB traffic
- **Cost**: ~€80/month

### High-Performance Configuration (Enterprise)
- **CPU**: 16 vCPUs
- **RAM**: 64 GB
- **Storage**: 1 TB SSD
- **Network**: 20 TB traffic
- **Cost**: ~€160/month

## 🚀 Step 1: Create Hetzner Server

### 1.1 Login to Hetzner Cloud Console
1. Go to [Hetzner Cloud Console](https://console.hetzner-cloud.com/)
2. Login with your credentials

### 1.2 Create New Project
1. Click "New Project"
2. Name it "IPPAN Blockchain"
3. Click "Create Project"

### 1.3 Create SSH Key
1. Go to "SSH Keys" in the left menu
2. Click "Add SSH Key"
3. Name it "IPPAN Server Key"
4. Paste your public SSH key
5. Click "Add SSH Key"

### 1.4 Create Server
1. Click "Add Server"
2. **Name**: `ippan-mainnet-node`
3. **Image**: Ubuntu 22.04
4. **Type**: Choose based on requirements above
5. **SSH Key**: Select your SSH key
6. **Location**: Choose closest to your users
7. **Networks**: Leave default
8. **Volumes**: Add additional storage if needed
9. **Firewalls**: Create new firewall (see below)
10. Click "Create & Buy Now"

### 1.5 Create Firewall
1. Go to "Firewalls" in left menu
2. Click "Create Firewall"
3. **Name**: `ippan-firewall`
4. **Rules**:
   - SSH (22): Allow from your IP
   - HTTP (80): Allow from anywhere
   - HTTPS (443): Allow from anywhere
   - IPPAN API (3000): Allow from anywhere
   - IPPAN P2P (8080): Allow from anywhere
   - Prometheus (9090): Allow from your IP only
   - Grafana (3001): Allow from your IP only
5. Click "Create Firewall"

## 🔧 Step 2: Server Setup

### 2.1 Connect to Server
```bash
# Replace with your server IP
ssh root@YOUR_SERVER_IP
```

### 2.2 Update System
```bash
# Update package list
apt update && apt upgrade -y

# Install essential packages
apt install -y curl wget git vim htop ufw fail2ban
```

### 2.3 Create IPPAN User
```bash
# Create dedicated user for IPPAN
adduser ippan
usermod -aG sudo ippan
usermod -aG docker ippan

# Switch to ippan user
su - ippan
```

### 2.4 Install Docker
```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Add user to docker group
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Verify installation
docker --version
docker-compose --version
```

### 2.5 Configure Firewall
```bash
# Configure UFW firewall
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 3000/tcp
sudo ufw allow 8080/tcp
sudo ufw allow 9090/tcp from YOUR_IP
sudo ufw allow 3001/tcp from YOUR_IP
sudo ufw enable
```

## 📁 Step 3: Deploy IPPAN

### 3.1 Clone Repository
```bash
# Clone IPPAN repository
git clone https://github.com/dmrl789/IPPAN.git
cd ippan

# Make scripts executable
chmod +x scripts/*.sh
```

### 3.2 Configure Environment
```bash
# Create production environment file
cp .env.example .env.mainnet

# Edit environment file
nano .env.mainnet
```

**Environment Configuration:**
```bash
# Database
POSTGRES_DB=ippan_mainnet
POSTGRES_USER=ippan
POSTGRES_PASSWORD=YOUR_SECURE_PASSWORD

# Redis
REDIS_PASSWORD=YOUR_SECURE_REDIS_PASSWORD

# JWT
JWT_SECRET=YOUR_JWT_SECRET_KEY

# Grafana
GRAFANA_ADMIN_PASSWORD=YOUR_GRAFANA_PASSWORD

# Backup
BACKUP_ENCRYPTION_KEY=YOUR_BACKUP_ENCRYPTION_KEY

# Network
IPPAN_NETWORK_PORT=8080
IPPAN_API_PORT=3000

# Production flags
NODE_ENV=production
IPPAN_ENVIRONMENT=mainnet
RUST_LOG=info
```

### 3.3 Create Directories
```bash
# Create mainnet directories
sudo mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups}
sudo chown -R ippan:ippan /opt/ippan
sudo chmod -R 755 /opt/ippan
```

### 3.4 Generate SSL Certificates
```bash
# Install Certbot for Let's Encrypt
sudo apt install -y certbot

# Generate SSL certificate (replace with your domain)
sudo certbot certonly --standalone -d your-domain.com

# Copy certificates to IPPAN directory
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem /opt/ippan/mainnet/ssl/
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem /opt/ippan/mainnet/ssl/
sudo chown -R ippan:ippan /opt/ippan/mainnet/ssl/
```

## 🚀 Step 4: Deploy Services

### 4.1 Build and Deploy
```bash
# Build Docker images
docker build -f Dockerfile.production -t ippan/ippan:latest .

# Deploy services
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml --env-file .env.mainnet up -d
```

### 4.2 Verify Deployment
```bash
# Check service status
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml ps

# Check logs
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml logs -f

# Test API endpoint
curl -f http://localhost:3000/api/v1/status
```

### 4.3 Configure Nginx (Optional)
```bash
# Install Nginx
sudo apt install -y nginx

# Create Nginx configuration
sudo nano /etc/nginx/sites-available/ippan
```

**Nginx Configuration:**
```nginx
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /api/ {
        proxy_pass http://localhost:3000/api/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/ippan /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

## 🔍 Step 5: Monitoring Setup

### 5.1 Access Monitoring
- **Prometheus**: `https://your-domain.com:9090`
- **Grafana**: `https://your-domain.com:3001`
- **IPPAN API**: `https://your-domain.com:3000`

### 5.2 Configure Grafana
1. Login to Grafana with admin credentials
2. Import IPPAN dashboards
3. Configure data sources
4. Set up alerts

### 5.3 Set Up Log Rotation
```bash
# Configure log rotation
sudo nano /etc/logrotate.d/ippan
```

**Log Rotation Configuration:**
```
/opt/ippan/mainnet/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 ippan ippan
    postrotate
        docker-compose -f /home/ippan/ippan/deployments/mainnet/docker-compose.mainnet.yml restart ippan-mainnet-node
    endscript
}
```

## 🔄 Step 6: Maintenance and Updates

### 6.1 Create Update Script
```bash
# Create update script
nano ~/update-ippan.sh
```

**Update Script:**
```bash
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
```

```bash
# Make executable
chmod +x ~/update-ippan.sh
```

### 6.2 Set Up Automated Backups
```bash
# Create backup script
nano ~/backup-ippan.sh
```

**Backup Script:**
```bash
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
```

```bash
# Make executable
chmod +x ~/backup-ippan.sh

# Add to crontab for daily backups
crontab -e
# Add this line:
# 0 2 * * * /home/ippan/backup-ippan.sh
```

## 🚨 Step 7: Security Hardening

### 7.1 Configure Fail2ban
```bash
# Configure fail2ban for SSH protection
sudo nano /etc/fail2ban/jail.local
```

**Fail2ban Configuration:**
```ini
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 3

[sshd]
enabled = true
port = ssh
logpath = /var/log/auth.log
maxretry = 3
```

```bash
# Restart fail2ban
sudo systemctl restart fail2ban
```

### 7.2 Disable Root Login
```bash
# Edit SSH configuration
sudo nano /etc/ssh/sshd_config
```

**SSH Configuration:**
```
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
```

```bash
# Restart SSH
sudo systemctl restart ssh
```

### 7.3 Set Up Monitoring Alerts
```bash
# Install monitoring tools
sudo apt install -y htop iotop nethogs

# Set up system monitoring
sudo nano /etc/systemd/system/ippan-monitor.service
```

**Monitoring Service:**
```ini
[Unit]
Description=IPPAN Monitoring
After=network.target

[Service]
Type=simple
User=ippan
ExecStart=/bin/bash -c 'while true; do echo "$(date): $(curl -s http://localhost:3000/api/v1/status | jq .status)" >> /opt/ippan/mainnet/logs/health.log; sleep 60; done'
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
# Enable monitoring
sudo systemctl enable ippan-monitor
sudo systemctl start ippan-monitor
```

## 📊 Step 8: Performance Optimization

### 8.1 System Optimization
```bash
# Optimize system parameters
sudo nano /etc/sysctl.conf
```

**System Parameters:**
```
# Network optimization
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 65536 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_congestion_control = bbr

# File system optimization
fs.file-max = 2097152
vm.swappiness = 10
```

```bash
# Apply changes
sudo sysctl -p
```

### 8.2 Docker Optimization
```bash
# Configure Docker daemon
sudo nano /etc/docker/daemon.json
```

**Docker Configuration:**
```json
{
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "100m",
    "max-file": "3"
  },
  "storage-driver": "overlay2",
  "storage-opts": [
    "overlay2.override_kernel_check=true"
  ]
}
```

```bash
# Restart Docker
sudo systemctl restart docker
```

## 🎉 Step 9: Go Live!

### 9.1 Final Verification
```bash
# Check all services
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml ps

# Test API endpoints
curl -f https://your-domain.com/api/v1/status
curl -f https://your-domain.com/api/v1/node/info

# Check monitoring
curl -f https://your-domain.com:9090/-/healthy
curl -f https://your-domain.com:3001/api/health
```

### 9.2 Performance Test
```bash
# Install Apache Bench for testing
sudo apt install -y apache2-utils

# Run performance test
ab -n 1000 -c 10 https://your-domain.com/api/v1/status
```

### 9.3 Announce Launch
1. Update DNS records to point to your server
2. Announce mainnet launch to community
3. Monitor system performance
4. Set up 24/7 monitoring

## 🔧 Troubleshooting

### Common Issues

#### Service Won't Start
```bash
# Check logs
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml logs

# Check system resources
htop
df -h
free -h
```

#### SSL Certificate Issues
```bash
# Renew certificates
sudo certbot renew

# Check certificate status
sudo certbot certificates
```

#### Performance Issues
```bash
# Check system performance
htop
iotop
nethogs

# Check Docker resources
docker stats
```

#### Network Issues
```bash
# Check firewall
sudo ufw status

# Check network connectivity
ping google.com
curl -I https://your-domain.com
```

## 📞 Support

### Useful Commands
```bash
# View all logs
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml logs -f

# Restart services
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml restart

# Update deployment
~/update-ippan.sh

# Create backup
~/backup-ippan.sh

# Check system status
systemctl status docker
systemctl status nginx
```

### Emergency Procedures
```bash
# Emergency stop
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml down

# Emergency restart
docker-compose -f deployments/mainnet/docker-compose.mainnet.yml restart

# Rollback from backup
tar -xzf /opt/ippan/mainnet/backups/ippan_backup_YYYYMMDD_HHMMSS.tar.gz
```

---

**🎊 Congratulations!** Your IPPAN blockchain is now running on Hetzner Cloud! 

For additional support or questions, refer to the IPPAN documentation or contact the development team.
