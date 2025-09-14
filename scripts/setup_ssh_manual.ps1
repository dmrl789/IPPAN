# Manual SSH Setup Script for IPPAN Servers

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$IPPAN_USER = "ippan"

Write-Host "=== IPPAN SSH Setup Instructions ===" -ForegroundColor Blue
Write-Host ""

# Display the SSH public key
$sshKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
if (Test-Path $sshKeyPath) {
    $publicKey = Get-Content $sshKeyPath
    Write-Host "Your SSH Public Key:" -ForegroundColor Green
    Write-Host $publicKey -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "=== Manual Setup Instructions ===" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Since the servers only accept public key authentication, you need to:" -ForegroundColor White
    Write-Host ""
    Write-Host "1. Access your server provider's console (Hetzner Cloud Console)" -ForegroundColor White
    Write-Host "2. Add the SSH key above to both servers" -ForegroundColor White
    Write-Host "3. Or recreate the servers with the updated cloud-init files" -ForegroundColor White
    Write-Host ""
    
    Write-Host "=== Alternative: Update Cloud-Init Files ===" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "I'll create updated cloud-init files with your SSH key..." -ForegroundColor White
    
    # Read the original cloud-init files
    $node1Content = Get-Content "ippan-cloud-init.yml" -Raw
    $node2Content = Get-Content "ippan-cloud-init-node2.yml" -Raw
    
    # Create updated versions with SSH key
    $sshKeySection = @"
  - name: ippan
    groups: [sudo, docker]
    shell: /bin/bash
    sudo: ['ALL=(ALL) NOPASSWD:ALL']
    lock_passwd: false
    passwd: `$6`$rounds=4096`$salt`$hashed_password_here
    ssh_authorized_keys:
      - $publicKey
"@

    # Update node1 cloud-init
    $node1Updated = $node1Content -replace "(users:\s*-\s*name:\s*ippan[^}]+})", $sshKeySection
    $node1Updated | Out-File -FilePath "ippan-cloud-init-node1-with-ssh.yml" -Encoding UTF8
    
    # Update node2 cloud-init  
    $node2Updated = $node2Content -replace "(users:\s*-\s*name:\s*ippan[^}]+})", $sshKeySection
    $node2Updated | Out-File -FilePath "ippan-cloud-init-node2-with-ssh.yml" -Encoding UTF8
    
    Write-Host "✅ Created updated cloud-init files:" -ForegroundColor Green
    Write-Host "  - ippan-cloud-init-node1-with-ssh.yml" -ForegroundColor Cyan
    Write-Host "  - ippan-cloud-init-node2-with-ssh.yml" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "=== Next Steps ===" -ForegroundColor Yellow
    Write-Host "1. Use the updated cloud-init files to recreate your servers" -ForegroundColor White
    Write-Host "2. Or manually add the SSH key via your server provider's console" -ForegroundColor White
    Write-Host "3. Once SSH access is working, run the deployment scripts" -ForegroundColor White
    Write-Host ""
    
    Write-Host "=== Test SSH Access ===" -ForegroundColor Yellow
    Write-Host "After setting up SSH access, test with:" -ForegroundColor White
    Write-Host "ssh $IPPAN_USER@$SERVER1_IP" -ForegroundColor Cyan
    Write-Host "ssh $IPPAN_USER@$SERVER2_IP" -ForegroundColor Cyan
    Write-Host ""
    
    Write-Host "=== Deploy IPPAN Services ===" -ForegroundColor Yellow
    Write-Host "Once SSH is working, run:" -ForegroundColor White
    Write-Host "./scripts/deploy_server1.sh" -ForegroundColor Cyan
    Write-Host "./scripts/deploy_server2_connect.sh" -ForegroundColor Cyan
    Write-Host ""
    
} else {
    Write-Host "❌ SSH key not found. Please run the diagnosis script first." -ForegroundColor Red
}
