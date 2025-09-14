# Deploy with PuTTY Keys
$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_PASSWORD = "LcNdL4Rsg3VU"
$SERVER2_PASSWORD = "Pam3C4dcwUq4"

Write-Host "=== Deploying with PuTTY Keys ===" -ForegroundColor Cyan
Write-Host ""

# First, let's try to use the standard SSH client with expect-like behavior
Write-Host "Trying alternative SSH approach..." -ForegroundColor Green

# Create a simple deployment script that uses expect-like behavior
$deployScript = @"
# Simple deployment script
apt update && apt upgrade -y
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true
mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs
chown -R ippan:ippan /opt/ippan
ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp
ufw --force enable
echo "Basic setup completed"
"@

# Save the script
$deployScript | Out-File -FilePath "basic_setup.sh" -Encoding UTF8

Write-Host "Created basic setup script" -ForegroundColor Green

# Try using ssh with a different approach
Write-Host "Testing Server 2 with password authentication..." -ForegroundColor Yellow

# Create a PowerShell script that handles SSH with password
$sshScript = @"
# SSH with password script
`$processInfo = New-Object System.Diagnostics.ProcessStartInfo
`$processInfo.FileName = "ssh"
`$processInfo.Arguments = "-o ConnectTimeout=10 -o StrictHostKeyChecking=no root@$SERVER2_IP"
`$processInfo.UseShellExecute = `$false
`$processInfo.RedirectStandardInput = `$true
`$processInfo.RedirectStandardOutput = `$true
`$processInfo.RedirectStandardError = `$true

`$process = New-Object System.Diagnostics.Process
`$process.StartInfo = `$processInfo
`$process.Start()

# Send password and commands
`$process.StandardInput.WriteLine("$SERVER2_PASSWORD")
`$process.StandardInput.WriteLine("bash -s")
`$process.StandardInput.Write((Get-Content "basic_setup.sh" -Raw))
`$process.StandardInput.WriteLine("exit")
`$process.StandardInput.Close()

`$output = `$process.StandardOutput.ReadToEnd()
`$error = `$process.StandardError.ReadToEnd()
`$process.WaitForExit()

Write-Host "Output: `$output"
if (`$error) { Write-Host "Error: `$error" }
"@

$sshScript | Out-File -FilePath "ssh_with_password.ps1" -Encoding UTF8

Write-Host ""
Write-Host "=== Manual Deployment Instructions ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Since automated SSH is having issues, here are the manual steps:" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Open two separate terminal windows" -ForegroundColor White
Write-Host ""
Write-Host "2. For Server 1 ($SERVER1_IP):" -ForegroundColor Green
Write-Host "   - Run: ssh root@$SERVER1_IP" -ForegroundColor Gray
Write-Host "   - Password: $SERVER1_PASSWORD" -ForegroundColor Gray
Write-Host "   - Copy and paste the contents of basic_setup.sh" -ForegroundColor Gray
Write-Host ""
Write-Host "3. For Server 2 ($SERVER2_IP):" -ForegroundColor Green
Write-Host "   - Run: ssh root@$SERVER2_IP" -ForegroundColor Gray
Write-Host "   - Password: $SERVER2_PASSWORD" -ForegroundColor Gray
Write-Host "   - Copy and paste the contents of basic_setup.sh" -ForegroundColor Gray
Write-Host ""
Write-Host "4. After basic setup, deploy IPPAN on both servers:" -ForegroundColor White
Write-Host "   - Run the IPPAN deployment commands we prepared earlier" -ForegroundColor Gray
Write-Host ""
Write-Host "5. Exit rescue mode using the API script" -ForegroundColor White
Write-Host "   - Run: powershell -ExecutionPolicy Bypass -File exit_rescue_mode.ps1" -ForegroundColor Gray
Write-Host ""
Write-Host "=== Basic Setup Script Contents ===" -ForegroundColor Cyan
Write-Host $deployScript -ForegroundColor Gray

Write-Host ""
Write-Host "=== Alternative: Try the SSH script ===" -ForegroundColor Yellow
Write-Host "You can also try running: powershell -ExecutionPolicy Bypass -File ssh_with_password.ps1" -ForegroundColor Gray
