# Copy IPPAN Deployment Commands to Clipboard

Write-Host "=== IPPAN Deployment Commands ===" -ForegroundColor Blue
Write-Host ""

# Server 1 commands
$server1Commands = @"
# Server 1 (Nuremberg - 188.245.97.41) Deployment Commands
sudo su - ippan
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "188.245.97.41:8080",  # Node 1 (Nuremberg)
    "135.181.145.174:8080" # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "188.245.97.41:8080"

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[metrics]
listen_address = "0.0.0.0:9090"

[logging]
level = "info"
format = "json"

[consensus]
algorithm = "proof_of_stake"
block_time = 10
max_transactions_per_block = 1000

[storage]
data_dir = "/opt/ippan/data"
wal_dir = "/opt/ippan/wal"
EOF

cd mainnet
docker-compose -f docker-compose.production.yml up -d
docker ps
"@

# Server 2 commands
$server2Commands = @"
# Server 2 (Helsinki - 135.181.145.174) Deployment Commands
sudo su - ippan
cd /opt/ippan
git clone https://github.com/dmrl789/IPPAN.git ippan-repo
cp -r ippan-repo/* mainnet/
rm -rf ippan-repo

cat > mainnet/config.toml << 'EOF'
[network]
bootstrap_nodes = [
    "188.245.97.41:8080",  # Node 1 (Nuremberg)
    "135.181.145.174:8080" # Node 2 (Helsinki)
]
listen_address = "0.0.0.0:8080"
external_address = "135.181.145.174:8080"

[api]
listen_address = "0.0.0.0:3000"
cors_origins = ["*"]

[metrics]
listen_address = "0.0.0.0:9090"

[logging]
level = "info"
format = "json"

[consensus]
algorithm = "proof_of_stake"
block_time = 10
max_transactions_per_block = 1000

[storage]
data_dir = "/opt/ippan/data"
wal_dir = "/opt/ippan/wal"
EOF

cd mainnet
docker-compose -f docker-compose.production.yml up -d
docker ps
"@

# Verification commands
$verificationCommands = @"
# Verification Commands (run on both servers)
curl http://localhost:3000/health
curl http://localhost:3000/api/v1/network/peers
docker-compose logs -f
"@

Write-Host "Choose which commands to copy:" -ForegroundColor Yellow
Write-Host "1. Server 1 (Nuremberg) commands" -ForegroundColor White
Write-Host "2. Server 2 (Helsinki) commands" -ForegroundColor White
Write-Host "3. Verification commands" -ForegroundColor White
Write-Host "4. All commands" -ForegroundColor White
Write-Host ""

$choice = Read-Host "Enter your choice (1-4)"

switch ($choice) {
    "1" {
        $server1Commands | Set-Clipboard
        Write-Host "✅ Server 1 commands copied to clipboard!" -ForegroundColor Green
        Write-Host "Paste these commands into the Hetzner console for Server 1 (188.245.97.41)" -ForegroundColor Cyan
    }
    "2" {
        $server2Commands | Set-Clipboard
        Write-Host "✅ Server 2 commands copied to clipboard!" -ForegroundColor Green
        Write-Host "Paste these commands into the Hetzner console for Server 2 (135.181.145.174)" -ForegroundColor Cyan
    }
    "3" {
        $verificationCommands | Set-Clipboard
        Write-Host "✅ Verification commands copied to clipboard!" -ForegroundColor Green
        Write-Host "Paste these commands into the Hetzner console to verify deployment" -ForegroundColor Cyan
    }
    "4" {
        $allCommands = @"
$server1Commands

$server2Commands

$verificationCommands
"@
        $allCommands | Set-Clipboard
        Write-Host "✅ All commands copied to clipboard!" -ForegroundColor Green
        Write-Host "Paste these commands into the Hetzner console for both servers" -ForegroundColor Cyan
    }
    default {
        Write-Host "❌ Invalid choice. Please run the script again and choose 1-4." -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Blue
Write-Host "1. Open Hetzner Cloud Console (https://console.hetzner.cloud/)" -ForegroundColor White
Write-Host "2. Open server console for the target server" -ForegroundColor White
Write-Host "3. Paste the commands from clipboard" -ForegroundColor White
Write-Host "4. Run the commands to deploy IPPAN" -ForegroundColor White
Write-Host "5. Repeat for the other server" -ForegroundColor White
Write-Host "6. Run verification commands to test the setup" -ForegroundColor White
