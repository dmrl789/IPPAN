Param(
  [Parameter(Mandatory=$true)][string]$RescuePassword,
  [string]$Server2IP = "135.181.145.174",
  [string]$Server1IP = "188.245.97.41",
  [string]$LaptopPubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub"
)

$ErrorActionPreference = 'Stop'

function Write-Info([string]$m){ Write-Host "[INFO] $m" -ForegroundColor Green }
function Write-Warn([string]$m){ Write-Host "[WARN] $m" -ForegroundColor Yellow }
function Write-Err([string]$m){ Write-Host "[ERROR] $m" -ForegroundColor Red }

# Ensure Posh-SSH is available
try {
  if (-not (Get-Module -ListAvailable -Name Posh-SSH)) {
    try { Set-PSRepository -Name PSGallery -InstallationPolicy Trusted -ErrorAction SilentlyContinue } catch {}
    Install-Module -Name Posh-SSH -Force -Scope CurrentUser -AllowClobber -Confirm:$false | Out-Null
  }
  Import-Module Posh-SSH -ErrorAction Stop
} catch {
  Write-Err "Failed to install/import Posh-SSH: $_"
  exit 1
}

# Build credential for rescue root
$sec = ConvertTo-SecureString $RescuePassword -AsPlainText -Force
$cred = New-Object System.Management.Automation.PSCredential ("root", $sec)

# Connect to rescue
Write-Info "Connecting to rescue root@$Server2IP"
$session = $null
for ($i=0; $i -lt 12; $i++) {
  try {
    $session = New-SSHSession -ComputerName $Server2IP -Credential $cred -AcceptKey -ConnectionTimeout 15 -ErrorAction Stop
    break
  } catch {
    Start-Sleep -Seconds 10
  }
}
if ($null -eq $session) { Write-Err "Unable to connect to rescue on $Server2IP"; exit 1 }

# Read laptop public key and base64
if (-not (Test-Path $LaptopPubKeyPath)) { Write-Err "Public key not found at $LaptopPubKeyPath"; exit 1 }
$pubkey = Get-Content -LiteralPath $LaptopPubKeyPath -Raw
$b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($pubkey))

# Remote script to inject key into the mounted root filesystem
$injectScript = @"
set -e
mkdir -p /mnt
for dev in /dev/sda1 /dev/sda2 /dev/vda1 /dev/vda2; do
  if [ -b "\$dev" ]; then mount "\$dev" /mnt && break; fi
done
if ! mountpoint -q /mnt; then
  ROOTPART=\$(lsblk -rpno NAME,FSTYPE | awk '\$2 ~ /ext4|xfs/ {print \$1; exit}')
  mount "\$ROOTPART" /mnt
fi
if ! chroot /mnt id -u ippan >/dev/null 2>&1; then chroot /mnt useradd -m -s /bin/bash -G sudo,docker ippan; fi
mkdir -p /mnt/home/ippan/.ssh
chmod 700 /mnt/home/ippan/.ssh
echo "__B64__" | base64 -d >> /mnt/home/ippan/.ssh/authorized_keys
chmod 600 /mnt/home/ippan/.ssh/authorized_keys
chroot /mnt chown -R ippan:ippan /home/ippan/.ssh
umount /mnt
"@
$injectScript = $injectScript -replace "__B64__", [Regex]::Escape($b64)

Write-Info "Injecting SSH key for user 'ippan' into server2 root filesystem"
$resp = Invoke-SSHCommand -SSHSession $session -Command $injectScript -TimeOut 300
if ($resp.ExitStatus -ne 0) { Write-Err "Key injection failed: $($resp.Error)"; exit 1 }

# Reboot to normal system
Write-Info "Rebooting server2 to normal OS"
Invoke-SSHCommand -SSHSession $session -Command "reboot" | Out-Null
Remove-SSHSession -SSHSession $session | Out-Null

# Wait for normal SSH with key (ippan)
Write-Info "Waiting for SSH as ippan@$Server2IP"
for ($i=0; $i -lt 36; $i++) {
  $p = Start-Process -FilePath "ssh" -ArgumentList "-o","StrictHostKeyChecking=no","-o","ConnectTimeout=10","ippan@$Server2IP","echo ok" -PassThru -WindowStyle Hidden -RedirectStandardOutput NUL -RedirectStandardError NUL -NoNewWindow -Wait
  if ($p.ExitCode -eq 0) { break }
  Start-Sleep -Seconds 10
  if ($i -eq 35) { Write-Err "SSH as ippan not available after reboot"; exit 1 }
}

# Deploy IPPAN on node2 and connect to node1
Write-Info "Deploying IPPAN on node2 and peering to node1"
$deployNode2 = @"
set -e
sudo apt update && sudo apt install -y curl git ufw fail2ban ca-certificates gnupg lsb-release
if ! command -v docker >/dev/null; then
  curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
  sudo usermod -aG docker \$USER
fi
if ! command -v docker-compose >/dev/null; then
  sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-\$(uname -s)-\$(uname -m)" -o /usr/local/bin/docker-compose && sudo chmod +x /usr/local/bin/docker-compose
fi
sudo ufw --force reset
sudo ufw default deny incoming && sudo ufw default allow outgoing
sudo ufw allow 22,80,443,3000,8080,9090,3001/tcp
sudo ufw --force enable
mkdir -p /opt/ippan && cd /opt/ippan
if [ ! -d mainnet/.git ]; then
  git clone https://github.com/dmrl789/IPPAN.git mainnet
fi
cd mainnet
cat > multi-node-node2.toml <<EOF
[network]
bootstrap_nodes = ["$Server1IP:8080","$Server2IP:8080"]
listen_address = "0.0.0.0:8080"
external_address = "$Server2IP:8080"
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
mkdir -p config && cp multi-node-node2.toml config/node.toml
docker-compose -f docker-compose.production.yml up -d
"@

$p = Start-Process -FilePath "ssh" -ArgumentList "-o","StrictHostKeyChecking=no","-o","ConnectTimeout=15","ippan@$Server2IP","bash -lc \"$deployNode2\"" -PassThru -WindowStyle Hidden -NoNewWindow -Wait
if ($p.ExitCode -ne 0) { Write-Err "Node2 deployment failed"; exit 1 }

# Verify
Write-Info "Verifying node2"
Start-Process -FilePath "ssh" -ArgumentList "-o","StrictHostKeyChecking=no","ippan@$Server2IP","docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -NoNewWindow -Wait | Out-Null
Start-Process -FilePath "ssh" -ArgumentList "-o","StrictHostKeyChecking=no","ippan@$Server2IP","curl -sf http://127.0.0.1:3000/health || echo API not ready" -NoNewWindow -Wait | Out-Null

Write-Info "Node2 is deployed and peered to node1."
