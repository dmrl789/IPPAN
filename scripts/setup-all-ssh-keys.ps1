# Display SSH key setup commands for all 4 servers
# Copy and run each command (enter password when prompted)

$pubKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan"

$servers = @(
    @{IP="188.245.97.41"; Pass="vK3n9MKjWb9XtTsVAttP"; Name="node1"},
    @{IP="135.181.145.174"; Pass="XhH7gUA7UM9gEPPALE7p"; Name="node2"},
    @{IP="5.223.51.238"; Pass="MriVKtEK9psU9RwMCidn"; Name="node3"},
    @{IP="178.156.219.107"; Pass="hPAtPLw7hx3ndKXTW4vM"; Name="node4"}
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "SSH Key Setup Commands" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Your SSH Public Key:" -ForegroundColor Yellow
Write-Host $pubKey -ForegroundColor White -BackgroundColor DarkBlue
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Run these commands (enter password when prompted):" -ForegroundColor Yellow
Write-Host "========================================`n" -ForegroundColor Cyan

foreach ($server in $servers) {
    Write-Host "# $($server.Name) ($($server.IP))" -ForegroundColor Green
    Write-Host "ssh root@$($server.IP) `"if ! id -u ippan >/dev/null 2>&1; then useradd -m -s /bin/bash ippan; usermod -aG sudo ippan; fi; mkdir -p /home/ippan/.ssh; echo '$pubKey' > /home/ippan/.ssh/authorized_keys; chmod 700 /home/ippan/.ssh; chmod 600 /home/ippan/.ssh/authorized_keys; chown -R ippan:ippan /home/ippan/.ssh; echo 'ippan ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/ippan; chmod 440 /etc/sudoers.d/ippan`"" -ForegroundColor White
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "After setup, verify with:" -ForegroundColor Yellow
Write-Host "========================================`n" -ForegroundColor Cyan

foreach ($server in $servers) {
    Write-Host "ssh ippan@$($server.IP) 'whoami && sudo -n true && echo SSH_AND_SUDO_OK'" -ForegroundColor White
}

Write-Host "`nThen run the deployment:" -ForegroundColor Cyan
Write-Host "powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\devnet1_hetzner_autodeploy.ps1" -ForegroundColor White

