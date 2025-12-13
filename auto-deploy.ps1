# Automated IPPAN Devnet-1 Deployment
# Uses root credentials to set up ippan user, then deploys

$ErrorActionPreference = "Continue"

# Server credentials
$servers = @{
    "188.245.97.41" = @{Pass="vK3n9MKjWb9XtTsVAttP"; PrivIP="10.0.0.2"; Name="node1"; Bootstrap=""}
    "135.181.145.174" = @{Pass="XhH7gUA7UM9gEPPALE7p"; PrivIP="10.0.0.3"; Name="node2"; Bootstrap="10.0.0.2"}
    "5.223.51.238" = @{Pass="MriVKtEK9psU9RwMCidn"; PrivIP=""; Name="node3"; Bootstrap="188.245.97.41"}
    "178.156.219.107" = @{Pass="hPAtPLw7hx3ndKXTW4vM"; PrivIP=""; Name="node4"; Bootstrap="188.245.97.41"}
}

# Get public key
$pubKeyPath = "$env:USERPROFILE\.ssh\id_ed25519.pub"
if (-not (Test-Path $pubKeyPath)) {
    $pubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
}
$pubKey = (Get-Content $pubKeyPath).Trim()

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "IPPAN Devnet-1 Automated Deployment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Function to execute SSH with password using expect-like approach
function Invoke-RootSSH {
    param(
        [string]$Host,
        [string]$Password,
        [string]$Command
    )
    
    # Try using ssh with password via here-string
    # Create a temporary script file
    $tempScript = [System.IO.Path]::GetTempFileName() + ".sh"
    $Command | Out-File -FilePath $tempScript -Encoding ASCII -NoNewline
    
    # Use ssh with password (requires sshpass or plink on Windows)
    # For now, use manual approach but provide automated alternative
    
    # Check if plink is available
    $plink = Get-Command plink -ErrorAction SilentlyContinue
    if ($plink) {
        Write-Host "Using plink for $Host..." -ForegroundColor Gray
        $result = & plink -ssh -pw $Password root@$Host $Command 2>&1
        return $result
    } else {
        # Fallback: create expect script or use manual method
        Write-Host "plink not found. Creating manual command..." -ForegroundColor Yellow
        Write-Host "Run: ssh root@$Host" -ForegroundColor White
        Write-Host "Password: $Password" -ForegroundColor Gray
        Write-Host "Command: $Command" -ForegroundColor White
        return $null
    }
}

Write-Host "STEP 1: Creating ippan user on all servers..." -ForegroundColor Yellow

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "`nSetting up $($server.Name) ($ip)..." -ForegroundColor Green
    
    $setupCmd = @"
if ! id -u ippan >/dev/null 2>&1; then
    useradd -m -s /bin/bash ippan
    usermod -aG sudo ippan
    echo 'ippan ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers.d/ippan
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
    echo 'User ippan created successfully'
else
    echo 'User ippan exists, updating SSH key'
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
    echo 'SSH key updated'
fi
"@
    
    $result = Invoke-RootSSH -Host $ip -Password $server.Pass -Command $setupCmd
    
    if ($result) {
        Write-Host "  Result: $result" -ForegroundColor Gray
    } else {
        Write-Host "  Manual setup required (see commands above)" -ForegroundColor Yellow
    }
}

Write-Host "`nVerifying ippan user setup..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "Testing SSH to $($server.Name)..." -ForegroundColor Gray
    $test = ssh -o ConnectTimeout=5 -o BatchMode=yes ippan@$ip "hostname" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ $($server.Name): SSH working" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($server.Name): SSH not working yet" -ForegroundColor Red
    }
}

Write-Host "`nSTEP 2: Deploying repository..." -ForegroundColor Yellow

# Phase 2: Copy repo
foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "`nDeploying to $($server.Name)..." -ForegroundColor Green
    
    $cmds = @(
        'sudo mkdir -p /opt/ippan && sudo chown -R ippan:ippan /opt/ippan',
        'cd /opt/ippan && (test -d .git && git fetch origin && git reset --hard origin/master || git clone https://github.com/dmrl789/IPPAN .) && git checkout master && git pull --ff-only',
        'cd /opt/ippan && chmod +x deploy/hetzner/scripts/setup-node.sh || true'
    )
    
    foreach ($cmd in $cmds) {
        Write-Host "  Running: $cmd" -ForegroundColor Gray
        $result = ssh ippan@$ip $cmd 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "    ✓ Success" -ForegroundColor Green
        } else {
            Write-Host "    ✗ Failed: $result" -ForegroundColor Red
        }
    }
}

Write-Host "`nSTEP 3: Running setup scripts (this will take 15-30 min per node)..." -ForegroundColor Yellow

foreach ($ip in $servers.Keys) {
    $server = $servers[$ip]
    Write-Host "`nSetting up $($server.Name)..." -ForegroundColor Green
    
    if ($server.Bootstrap) {
        $setupCmd = "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name) $($server.Bootstrap)"
    } else {
        $setupCmd = "cd /opt/ippan && ./deploy/hetzner/scripts/setup-node.sh $($server.Name)"
    }
    
    Write-Host "  Running setup script (this may take 15-30 minutes)..." -ForegroundColor Yellow
    Write-Host "  Command: $setupCmd" -ForegroundColor Gray
    
    # Run in background or provide manual command
    Write-Host "  Run manually: ssh ippan@$ip '$setupCmd'" -ForegroundColor White
}

Write-Host "`nAfter setup scripts complete, continue with Phase 4-7 from RUN_DEPLOYMENT.md" -ForegroundColor Cyan

