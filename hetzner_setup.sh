#!/bin/bash
set -e

echo "🚀 Starting IPPAN Hetzner Server Setup..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Step 1: Update System
print_status "Updating system packages..."
apt update && apt upgrade -y

# Step 2: Install Essential Packages
print_status "Installing essential packages..."
apt install -y curl wget git vim htop ufw fail2ban

# Step 3: Create IPPAN User
print_status "Creating IPPAN user..."
if ! id "ippan" &>/dev/null; then
    adduser --disabled-password --gecos "" ippan
    usermod -aG sudo ippan
    print_status "IPPAN user created successfully"
else
    print_warning "IPPAN user already exists"
fi

# Step 4: Install Docker
print_status "Installing Docker..."
if ! command -v docker &> /dev/null; then
    curl -fsSL https://get.docker.com -o get-docker.sh
    sh get-docker.sh
    rm get-docker.sh
    print_status "Docker installed successfully"
else
    print_warning "Docker already installed"
fi

# Step 5: Install Docker Compose
print_status "Installing Docker Compose..."
if ! command -v docker-compose &> /dev/null; then
    curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
    chmod +x /usr/local/bin/docker-compose
    print_status "Docker Compose installed successfully"
else
    print_warning "Docker Compose already installed"
fi

# Step 6: Configure Firewall
print_status "Configuring UFW firewall..."
ufw --force reset
ufw default deny incoming
ufw default allow outgoing

# Allow SSH from your IP only
ufw allow from 195.231.121.192 to any port 22

# Allow HTTP and HTTPS
ufw allow 80/tcp
ufw allow 443/tcp

# Allow IPPAN services
ufw allow 3000/tcp  # IPPAN API
ufw allow 8080/tcp  # IPPAN P2P

# Allow monitoring from your IP only
ufw allow from 195.231.121.192 to any port 9090  # Prometheus
ufw allow from 195.231.121.192 to any port 3001  # Grafana

# Enable firewall
ufw --force enable

print_status "Firewall configured successfully"

# Step 7: Configure Fail2ban
print_status "Configuring Fail2ban..."
cat > /etc/fail2ban/jail.local << EOF
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 3

[sshd]
enabled = true
port = ssh
logpath = /var/log/auth.log
maxretry = 3
EOF

systemctl restart fail2ban
systemctl enable fail2ban

print_status "Fail2ban configured successfully"

# Step 8: Create IPPAN directories
print_status "Creating IPPAN directories..."
mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl}
chown -R ippan:ippan /opt/ippan
chmod -R 755 /opt/ippan

print_status "IPPAN directories created successfully"

# Step 9: Verify Installation
print_status "Verifying installation..."
echo "Docker version: $(docker --version)"
echo "Docker Compose version: $(docker-compose --version)"
echo "UFW status:"
ufw status verbose

print_status "🎉 Server setup completed successfully!"
print_status "You can now proceed with IPPAN deployment"
print_status "Next steps:"
echo "1. Switch to ippan user: su - ippan"
echo "2. Clone IPPAN repository"
echo "3. Configure environment variables"
echo "4. Deploy IPPAN services"
