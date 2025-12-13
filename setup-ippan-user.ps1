# Setup ippan user on all Hetzner servers
# This script creates the ippan user and sets up SSH key authentication

$servers = @(
    @{IP="188.245.97.41"; Pass="vK3n9MKjWb9XtTsVAttP"; Name="node1"},
    @{IP="135.181.145.174"; Pass="XhH7gUA7UM9gEPPALE7p"; Name="node2"},
    @{IP="5.223.51.238"; Pass="MriVKtEK9psU9RwMCidn"; Name="node3"},
    @{IP="178.156.219.107"; Pass="hPAtPLw7hx3ndKXTW4vM"; Name="node4"}
)

# Get public key
$pubKeyPath = "$env:USERPROFILE\.ssh\id_ed25519.pub"
if (-not (Test-Path $pubKeyPath)) {
    $pubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
}
$pubKey = Get-Content $pubKeyPath

Write-Host "Setting up ippan user on all servers..." -ForegroundColor Cyan

foreach ($server in $servers) {
    Write-Host "`nSetting up $($server.Name) ($($server.IP))..." -ForegroundColor Yellow
    
    # Create ippan user and add to sudoers
    $setupUser = @"
#!/bin/bash
if ! id -u ippan >/dev/null 2>&1; then
    useradd -m -s /bin/bash ippan
    usermod -aG sudo ippan
    echo 'ippan ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers.d/ippan
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' > /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
    echo "User ippan created successfully"
else
    echo "User ippan already exists"
    mkdir -p /home/ippan/.ssh
    echo '$pubKey' >> /home/ippan/.ssh/authorized_keys
    chmod 700 /home/ippan/.ssh
    chmod 600 /home/ippan/.ssh/authorized_keys
    chown -R ippan:ippan /home/ippan/.ssh
fi
"@
    
    # Save script temporarily
    $tempScript = [System.IO.Path]::GetTempFileName()
    $setupUser | Out-File -FilePath $tempScript -Encoding ASCII
    
    Write-Host "  Creating ippan user..." -ForegroundColor Gray
    # Use plink or ssh with password - for now, provide manual command
    Write-Host "  Run this command manually (enter password when prompted):" -ForegroundColor Yellow
    Write-Host "  ssh root@$($server.IP) 'bash -s' < $tempScript" -ForegroundColor White
    
    # Try with expect-like approach or just provide the commands
    Write-Host "  Or run these commands:" -ForegroundColor Yellow
    Write-Host "  ssh root@$($server.IP)" -ForegroundColor White
    Write-Host "  Then paste:" -ForegroundColor Gray
    Write-Host $setupUser -ForegroundColor Cyan
}

Write-Host "`nAfter setting up users, verify SSH works:" -ForegroundColor Green
foreach ($server in $servers) {
    Write-Host "  ssh ippan@$($server.IP) 'hostname'" -ForegroundColor White
}

