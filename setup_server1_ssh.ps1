# Setup SSH Access for Server 1
# This script provides instructions to manually add SSH key to Server 1

$SERVER1_IP = "188.245.97.41"
$IPPAN_USER = "ippan"

Write-Host "=== Server 1 SSH Setup Instructions ===" -ForegroundColor Cyan
Write-Host ""

# Get the SSH public key
$sshKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
if (Test-Path $sshKeyPath) {
    $publicKey = Get-Content $sshKeyPath
    Write-Host "Your SSH Public Key:" -ForegroundColor Green
    Write-Host $publicKey -ForegroundColor Yellow
    Write-Host ""
} else {
    Write-Host "SSH key not found. Please generate one first." -ForegroundColor Red
    exit 1
}

Write-Host "=== Manual Setup Steps ===" -ForegroundColor Blue
Write-Host ""
Write-Host "1. Go to Hetzner Cloud Console: https://console.hetzner.cloud/" -ForegroundColor White
Write-Host "2. Find Server 1 ($SERVER1_IP) and click on it" -ForegroundColor White
Write-Host "3. Click the 'Console' tab" -ForegroundColor White
Write-Host "4. Login as root (or available user)" -ForegroundColor White
Write-Host "5. Run these commands in the console:" -ForegroundColor White
Write-Host ""

$setupCommands = @"
# Create ippan user if it doesn't exist
useradd -m -s /bin/bash -G sudo,docker ippan

# Create SSH directory
mkdir -p /home/ippan/.ssh

# Add your SSH public key
echo '$publicKey' >> /home/ippan/.ssh/authorized_keys

# Set proper permissions
chown -R ippan:ippan /home/ippan/.ssh
chmod 700 /home/ippan/.ssh
chmod 600 /home/ippan/.ssh/authorized_keys

# Install Docker if not already installed
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
usermod -aG docker ippan

# Create IPPAN directory
mkdir -p /opt/ippan/mainnet
chown -R ippan:ippan /opt/ippan
"@

Write-Host $setupCommands -ForegroundColor Cyan
Write-Host ""

Write-Host "6. After running the commands above, test SSH access:" -ForegroundColor White
Write-Host "   ssh $IPPAN_USER@$SERVER1_IP" -ForegroundColor Yellow
Write-Host ""

Write-Host "7. If SSH works, we can then deploy IPPAN services!" -ForegroundColor Green
Write-Host ""

# Save the commands to a file for easy copying
$setupCommands | Out-File -FilePath "server1_setup_commands.txt" -Encoding UTF8
Write-Host "Commands saved to: server1_setup_commands.txt" -ForegroundColor Green
Write-Host "You can copy and paste these commands into the Hetzner console." -ForegroundColor Green
