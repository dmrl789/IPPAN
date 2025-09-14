# Connect to Server 1 in Rescue Mode and Set Up IPPAN
# Server 1: 188.245.97.41
# Rescue credentials: root / 7LuR4nUCfTiv

$SERVER1_IP = "188.245.97.41"
$RESCUE_PASSWORD = "7LuR4nUCfTiv"
$IPPAN_USER = "ippan"

Write-Host "=== Server 1 Rescue Mode Setup ===" -ForegroundColor Cyan
Write-Host "Server: $SERVER1_IP" -ForegroundColor Blue
Write-Host "Rescue User: root" -ForegroundColor Blue
Write-Host ""

# Get SSH public key
$sshKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
if (Test-Path $sshKeyPath) {
    $publicKey = Get-Content $sshKeyPath
    Write-Host "SSH Public Key:" -ForegroundColor Green
    Write-Host $publicKey -ForegroundColor Yellow
    Write-Host ""
} else {
    Write-Host "SSH key not found!" -ForegroundColor Red
    exit 1
}

Write-Host "=== Step 1: Connect to Server 1 ===" -ForegroundColor Blue
Write-Host "Use these credentials to connect:" -ForegroundColor White
Write-Host "ssh root@$SERVER1_IP" -ForegroundColor Yellow
Write-Host "Password: $RESCUE_PASSWORD" -ForegroundColor Yellow
Write-Host ""

Write-Host "=== Step 2: Run Setup Commands ===" -ForegroundColor Blue
Write-Host "Copy and paste these commands in the SSH session:" -ForegroundColor White
Write-Host ""

$setupCommands = @"
# Update system
apt update && apt upgrade -y

# Install essential packages
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh

# Create ippan user
useradd -m -s /bin/bash -G sudo,docker ippan

# Create SSH directory for ippan user
mkdir -p /home/ippan/.ssh

# Add SSH public key
echo '$publicKey' >> /home/ippan/.ssh/authorized_keys

# Set proper permissions
chown -R ippan:ippan /home/ippan/.ssh
chmod 700 /home/ippan/.ssh
chmod 600 /home/ippan/.ssh/authorized_keys

# Create IPPAN directories
mkdir -p /opt/ippan/mainnet
mkdir -p /opt/ippan/data
mkdir -p /opt/ippan/keys
mkdir -p /opt/ippan/logs
chown -R ippan:ippan /opt/ippan

# Configure firewall
ufw allow 22/tcp    # SSH
ufw allow 3000/tcp  # API
ufw allow 8080/tcp  # P2P
ufw allow 9090/tcp  # Prometheus
ufw allow 3001/tcp  # Grafana
ufw --force enable

# Exit rescue mode and boot normally
exit
"@

Write-Host $setupCommands -ForegroundColor Cyan
Write-Host ""

Write-Host "=== Step 3: Exit Rescue Mode ===" -ForegroundColor Blue
Write-Host "After running the commands above:" -ForegroundColor White
Write-Host "1. Type 'exit' to close the SSH session" -ForegroundColor Yellow
Write-Host "2. Go to Hetzner Cloud Console" -ForegroundColor Yellow
Write-Host "3. Find Server 1 and click 'Actions' -> 'Exit Rescue Mode'" -ForegroundColor Yellow
Write-Host "4. Wait for the server to reboot normally (2-3 minutes)" -ForegroundColor Yellow
Write-Host ""

Write-Host "=== Step 4: Test SSH Access ===" -ForegroundColor Blue
Write-Host "After the server reboots, test SSH access:" -ForegroundColor White
Write-Host "ssh $IPPAN_USER@$SERVER1_IP" -ForegroundColor Yellow
Write-Host ""

Write-Host "=== Step 5: Deploy IPPAN Services ===" -ForegroundColor Blue
Write-Host "Once SSH is working, we'll deploy IPPAN services!" -ForegroundColor Green
Write-Host ""

# Save commands to file
$setupCommands | Out-File -FilePath "server1_rescue_setup_commands.txt" -Encoding UTF8
Write-Host "Commands saved to: server1_rescue_setup_commands.txt" -ForegroundColor Green
