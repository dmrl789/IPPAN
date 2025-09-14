Param(
  [Parameter(Mandatory=$true)][string]$Token,
  [Parameter(Mandatory=$true)][string]$Server1IP,
  [Parameter(Mandatory=$true)][string]$Server2IP,
  [string]$SshPubKeyPath = "$env:USERPROFILE\.ssh\id_rsa.pub",
  [string]$HetznerKeyName = "LaptopKey"
)

$ErrorActionPreference = 'Stop'

function Write-Info([string]$m){ Write-Host "[INFO] $m" -ForegroundColor Green }
function Write-Warn([string]$m){ Write-Host "[WARN] $m" -ForegroundColor Yellow }
function Write-Err([string]$m){ Write-Host "[ERROR] $m" -ForegroundColor Red }

$BaseUrl = 'https://api.hetzner.cloud/v1'
$Headers = @{ 'Authorization' = "Bearer $Token"; 'Content-Type' = 'application/json' }

function Invoke-HcloudApi([string]$Method, [string]$Path, $BodyObj){
  $Uri = "$BaseUrl$Path"
  if ($null -ne $BodyObj){
    $Body = ($BodyObj | ConvertTo-Json -Depth 10)
    return Invoke-RestMethod -Method $Method -Uri $Uri -Headers $Headers -Body $Body
  } else {
    return Invoke-RestMethod -Method $Method -Uri $Uri -Headers $Headers
  }
}

function Get-OrCreateSshKey([string]$KeyName, [string]$PublicKey){
  Write-Info "Ensuring SSH key '$KeyName' exists in Hetzner account"
  $resp = Invoke-HcloudApi GET "/ssh_keys" $null
  $existing = $resp.ssh_keys | Where-Object { $_.name -eq $KeyName }
  if ($null -ne $existing){
    Write-Info "SSH key exists (id=$($existing.id))"
    return $existing
  }
  Write-Info "Creating SSH key '$KeyName'"
  $create = Invoke-HcloudApi POST "/ssh_keys" @{ name=$KeyName; public_key=$PublicKey }
  return $create.ssh_key
}

function Get-ServerByIP([string]$ip){
  $resp = Invoke-HcloudApi GET "/servers" $null
  foreach($s in $resp.servers){
    if ($s.public_net -and $s.public_net.ipv4 -and $s.public_net.ipv4.ip -eq $ip){ return $s }
  }
  return $null
}

function Wait-ForSsh([string]$user, [string]$ip, [int]$timeoutSec=300){
  $start = Get-Date
  while ((Get-Date) - $start -lt [TimeSpan]::FromSeconds($timeoutSec)){
    try {
      $p = Start-Process -FilePath "ssh" -ArgumentList "-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=10", "$user@$ip", "echo ok" -Wait -NoNewWindow -PassThru -RedirectStandardOutput NUL -RedirectStandardError NUL
      if ($p.ExitCode -eq 0){ return $true }
    } catch {}
    Start-Sleep -Seconds 5
  }
  return $false
}

function Enable-Rescue([int]$serverId, [int]$sshKeyId){
  Write-Info "Enable rescue for serverId=$serverId"
  $body = @{ type = 'linux'; ssh_keys = @($sshKeyId) }
  Invoke-HcloudApi POST "/servers/$serverId/actions/enable_rescue" $body | Out-Null
}

function Disable-Rescue([int]$serverId){
  Write-Info "Disable rescue for serverId=$serverId"
  Invoke-HcloudApi POST "/servers/$serverId/actions/disable_rescue" $null | Out-Null
}

function Reboot-Server([int]$serverId){
  Write-Info "Reboot serverId=$serverId"
  Invoke-HcloudApi POST "/servers/$serverId/actions/reboot" $null | Out-Null
}

function Inject-Key-In-Rescue([string]$ip, [string]$pubKey){
  Write-Info "Injecting SSH key into system on $ip via rescue"
  $escapedKey = $pubKey.Replace('"','\"')
  $script = @'
set -e
mkdir -p /mnt
# Try typical root partitions, fallback to auto-detect ext4/xfs
for dev in /dev/sda1 /dev/sda2 /dev/vda1 /dev/vda2; do
  if [ -b "$dev" ]; then mount "$dev" /mnt && break; fi
done
if ! mountpoint -q /mnt; then
  ROOTPART=$(lsblk -rpno NAME,FSTYPE | awk '$2 ~ /ext4|xfs/ {print $1; exit}')
  mount "$ROOTPART" /mnt
fi
# Ensure user exists
if ! chroot /mnt id -u ippan >/dev/null 2>&1; then chroot /mnt useradd -m -s /bin/bash -G sudo,docker ippan; fi
# Place key
mkdir -p /mnt/home/ippan/.ssh
chmod 700 /mnt/home/ippan/.ssh
printf "%s\n" "__PUBKEY__" >> /mnt/home/ippan/.ssh/authorized_keys
chmod 600 /mnt/home/ippan/.ssh/authorized_keys
chroot /mnt chown -R ippan:ippan /home/ippan/.ssh
umount /mnt
'@
  $script = $script.Replace("__PUBKEY__", $escapedKey)
  # Stream script over SSH via stdin to avoid quoting/heredoc issues
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = "ssh"
  $psi.Arguments = "-o StrictHostKeyChecking=no -o ConnectTimeout=15 root@$ip bash -s"
  $psi.RedirectStandardInput = $true
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.UseShellExecute = $false
  $proc = [System.Diagnostics.Process]::Start($psi)
  $proc.StandardInput.Write($script)
  $proc.StandardInput.Close()
  $proc.WaitForExit()
  if ($proc.ExitCode -ne 0) { throw "Remote injection script failed on $ip" }
}

# --- MAIN ---
Write-Info "Reading public key from $SshPubKeyPath"
if (-not (Test-Path $SshPubKeyPath)) { throw "Public key not found at $SshPubKeyPath" }
$PublicKey = Get-Content -LiteralPath $SshPubKeyPath -Raw

$key = Get-OrCreateSshKey -KeyName $HetznerKeyName -PublicKey $PublicKey

$server1 = Get-ServerByIP -ip $Server1IP
$server2 = Get-ServerByIP -ip $Server2IP
if ($null -eq $server1) { throw "Server with IP $Server1IP not found in Hetzner project" }
if ($null -eq $server2) { throw "Server with IP $Server2IP not found in Hetzner project" }
Write-Info "Server1 id=$($server1.id) name=$($server1.name)"
Write-Info "Server2 id=$($server2.id) name=$($server2.name)"

foreach($pair in @(@{ip=$Server1IP; id=$server1.id}, @{ip=$Server2IP; id=$server2.id})){
  $ip = $pair.ip; $id = [int]$pair.id
  Write-Info "Processing $ip (id=$id)"
  Enable-Rescue -serverId $id -sshKeyId $key.id
  Reboot-Server -serverId $id
  Write-Info "Waiting for rescue SSH (root@$ip)"
  if (-not (Wait-ForSsh -user 'root' -ip $ip -timeoutSec 420)) { throw "Rescue SSH not available for $ip" }
  Inject-Key-In-Rescue -ip $ip -pubKey $PublicKey
  Disable-Rescue -serverId $id
  Reboot-Server -serverId $id
  Write-Info "Waiting for normal SSH (ippan@$ip)"
  if (-not (Wait-ForSsh -user 'ippan' -ip $ip -timeoutSec 420)) { throw "SSH for ippan@$ip not available after rescue" }
  Write-Info "SSH verified: ippan@$ip"
}

Write-Info "Both servers are accessible via SSH as ippan. You can now deploy."
