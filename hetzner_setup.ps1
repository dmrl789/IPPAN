# IPPAN Hetzner Server Setup Script
# Run this script to automate the server setup process

param(
    [string]$ServerIP = "188.245.97.41",
    [string]$SSHKeyPath = "$env:USERPROFILE\.ssh\id_rsa_ippan"
)

Write-Host "🚀 Starting IPPAN Hetzner Server Setup..." -ForegroundColor Green

# Function to execute SSH commands
function Invoke-SSHCommand {
    param([string]$Command)
    
    Write-Host "Executing: $Command" -ForegroundColor Yellow
    ssh -i $SSHKeyPath root@$ServerIP $Command
}

try {
    # Step 1: Update System
    Write-Host "📦 Updating system packages..." -ForegroundColor Green
    Invoke-SSHCommand "apt update && apt upgrade -y"
    
    # Step 2: Install Essential Packages
    Write-Host "📦 Installing essential packages..." -ForegroundColor Green
    Invoke-SSHCommand "apt install -y curl wget git vim htop ufw fail2ban"
    
    # Step 3: Create IPPAN User
    Write-Host "👤 Creating IPPAN user..." -ForegroundColor Green
    Invoke-SSHCommand "adduser --disabled-password --gecos '' ippan"
    Invoke-SSHCommand "usermod -aG sudo ippan"
    
    # Step 4: Install Docker
    Write-Host "🐳 Installing Docker..." -ForegroundColor Green
    Invoke-SSHCommand "curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh"
    
    # Step 5: Install Docker Compose
    Write-Host "🐳 Installing Docker Compose..." -ForegroundColor Green
    Invoke-SSHCommand "curl -L 'https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)' -o /usr/local/bin/docker-compose && chmod +x /usr/local/bin/docker-compose"
    
    # Step 6: Configure Firewall
    Write-Host "🔥 Configuring firewall..." -ForegroundColor Green
    Invoke-SSHCommand "ufw --force reset"
    Invoke-SSHCommand "ufw default deny incoming && ufw default allow outgoing"
    Invoke-SSHCommand "ufw allow from 195.231.121.192 to any port 22"
    Invoke-SSHCommand "ufw allow 80/tcp && ufw allow 443/tcp"
    Invoke-SSHCommand "ufw allow 3000/tcp && ufw allow 8080/tcp"
    Invoke-SSHCommand "ufw allow from 195.231.121.192 to any port 9090"
    Invoke-SSHCommand "ufw allow from 195.231.121.192 to any port 3001"
    Invoke-SSHCommand "ufw --force enable"
    
    # Step 7: Create IPPAN directories
    Write-Host "📁 Creating IPPAN directories..." -ForegroundColor Green
    Invoke-SSHCommand "mkdir -p /opt/ippan/mainnet/{data,keys,logs,monitor,grafana,alertmanager,postgres,redis,nginx/logs,fluentd,backups,ssl}"
    Invoke-SSHCommand "chown -R ippan:ippan /opt/ippan && chmod -R 755 /opt/ippan"
    
    # Step 8: Verify Installation
    Write-Host "✅ Verifying installation..." -ForegroundColor Green
    Invoke-SSHCommand "docker --version"
    Invoke-SSHCommand "docker-compose --version"
    Invoke-SSHCommand "ufw status verbose"
    
    Write-Host "🎉 Server setup completed successfully!" -ForegroundColor Green
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "1. Connect to server: ssh -i $SSHKeyPath root@$ServerIP" -ForegroundColor White
    Write-Host "2. Switch to ippan user: su - ippan" -ForegroundColor White
    Write-Host "3. Clone IPPAN repository" -ForegroundColor White
    Write-Host "4. Configure environment variables" -ForegroundColor White
    Write-Host "5. Deploy IPPAN services" -ForegroundColor White
    
} catch {
    Write-Host "❌ Error during setup: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
