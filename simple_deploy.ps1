# Simple Deployment using direct SSH commands
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_PASSWORD = "4CR9d33HHKAM"
$SERVER2_PASSWORD = "ijN97hEsePEr"

Write-Host "=== Simple IPPAN Deployment ===" -ForegroundColor Cyan
Write-Host ""

# Try to use plink (PuTTY Link) for password authentication
Write-Host "Checking for plink..." -ForegroundColor Green
if (Get-Command plink -ErrorAction SilentlyContinue) {
    Write-Host "✅ Plink found, using it for deployment" -ForegroundColor Green
    
    # Deploy to Server 1
    Write-Host "Deploying to Server 1..." -ForegroundColor Yellow
    $server1Script = @"
apt update && apt upgrade -y
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true
mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs
chown -R ippan:ippan /opt/ippan
ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp
ufw --force enable
su - ippan -c 'cd /opt/ippan && git clone https://github.com/dmrl789/IPPAN.git ippan-repo && cp -r ippan-repo/* mainnet/ && rm -rf ippan-repo'
echo "Basic setup completed on Server 1"
"@
    
    $server1Script | Out-File -FilePath "server1_setup.sh" -Encoding UTF8
    echo y | plink -ssh -pw $SERVER1_PASSWORD root@$SERVER1_IP -m server1_setup.sh
    
    # Deploy to Server 2
    Write-Host "Deploying to Server 2..." -ForegroundColor Yellow
    $server2Script = @"
apt update && apt upgrade -y
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true
mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs
chown -R ippan:ippan /opt/ippan
ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp
ufw --force enable
su - ippan -c 'cd /opt/ippan && git clone https://github.com/dmrl789/IPPAN.git ippan-repo && cp -r ippan-repo/* mainnet/ && rm -rf ippan-repo'
echo "Basic setup completed on Server 2"
"@
    
    $server2Script | Out-File -FilePath "server2_setup.sh" -Encoding UTF8
    echo y | plink -ssh -pw $SERVER2_PASSWORD root@$SERVER2_IP -m server2_setup.sh
    
    Write-Host "✅ Basic setup completed on both servers" -ForegroundColor Green
    
} else {
    Write-Host "❌ Plink not found. Please install PuTTY or use manual deployment." -ForegroundColor Red
    Write-Host ""
    Write-Host "Manual deployment instructions:" -ForegroundColor Yellow
    Write-Host "Server 1: ssh root@$SERVER1_IP (password: $SERVER1_PASSWORD)" -ForegroundColor White
    Write-Host "Server 2: ssh root@$SERVER2_IP (password: $SERVER2_PASSWORD)" -ForegroundColor White
    Write-Host ""
    Write-Host "Then run the deployment commands from the scripts we created earlier." -ForegroundColor Gray
}

Write-Host ""
Write-Host "=== Exiting Rescue Mode ===" -ForegroundColor Cyan

# Exit rescue mode using API
$HETZNER_API_TOKEN = "vdpFTnRJdXjlz24rsgNAIS3sUwfrz4gBUkSSmu69xrj7N430Q977LSB8QEUy3QnJ"
$SERVER1_ID = "108447288"
$SERVER2_ID = "108535607"

$headers = @{
    "Authorization" = "Bearer $HETZNER_API_TOKEN"
    "Content-Type" = "application/json"
}

Write-Host "Exiting rescue mode on Server 1..." -ForegroundColor Green
try {
    $exitResponse1 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER1_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 1" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 1: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "Exiting rescue mode on Server 2..." -ForegroundColor Green
try {
    $exitResponse2 = Invoke-RestMethod -Uri "https://api.hetzner.cloud/v1/servers/$SERVER2_ID/actions/disable_rescue" -Headers $headers -Method POST
    Write-Host "✅ Rescue mode disabled on Server 2" -ForegroundColor Green
} catch {
    Write-Host "❌ Failed to exit rescue mode on Server 2: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "Waiting for servers to restart..." -ForegroundColor Yellow
Start-Sleep -Seconds 60

Write-Host ""
Write-Host "=== Testing Final Deployment ===" -ForegroundColor Cyan

# Test APIs
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 1 API responding: $($api1.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 1 API not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    Write-Host "✅ Server 2 API responding: $($api2.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ Server 2 API not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Green
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
Write-Host ""
Write-Host "Both servers should now be connected as peers in the IPPAN network!" -ForegroundColor Green

# Clean up temp files
Remove-Item -Path "server1_setup.sh" -ErrorAction SilentlyContinue
Remove-Item -Path "server2_setup.sh" -ErrorAction SilentlyContinue
