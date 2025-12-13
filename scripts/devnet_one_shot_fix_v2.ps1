# scripts/devnet_one_shot_fix_v2.ps1
# Robust one-shot: send bash scripts via stdin (no nested quoting), capture evidence, apply systemd+dirs fix, restart, verify.
# No loops. No Docker. No password changes.

$ErrorActionPreference = "Continue"

function NowStamp { (Get-Date).ToString("yyyyMMdd_HHmmss") }

$STAMP = NowStamp
$BUNDLE = Join-Path "tmp/devnet" ("one_shot_fix_v2_" + $STAMP)
New-Item -ItemType Directory -Force -Path $BUNDLE | Out-Null

$SSH_USER = "ippan-devnet"

$NODES = @(
  @{ name="node1"; host="188.245.97.41"; role="bootstrap" },
  @{ name="node2"; host="135.181.145.174"; role="validator" },
  @{ name="node3"; host="5.223.51.238";  role="validator" },
  @{ name="node4"; host="178.156.219.107"; role="observer" }
)

# Best-effort: your public IP for node4 RPC allowlist
$MYIP = ""
try { $MYIP = (curl.exe -fsS https://api.ipify.org).Trim() } catch { $MYIP = "" }

function WriteBundle($nodeName, $fileName, $content) {
  $dir = Join-Path $BUNDLE $nodeName
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  $path = Join-Path $dir $fileName
  $content | Out-File -FilePath $path -Encoding utf8
}

function RunRemoteScript($targetHost, $scriptText, $asSudo) {
  # Avoid PowerShell/ssh quoting: we pipe a plain bash script into (sudo) bash -s
  $scriptText = $scriptText -replace "`r`n", "`n"
  $tmp = Join-Path $BUNDLE ("_payload_" + $targetHost.Replace(".","_") + ".sh")
  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($tmp, $scriptText, $utf8NoBom)

  $stdin = Get-Content -Raw -Encoding utf8 $tmp
  try {
    if ($asSudo) {
      $out = $stdin | & ssh.exe -o BatchMode=yes -o StrictHostKeyChecking=accept-new "$SSH_USER@$targetHost" "sudo -n bash -s" 2>&1
    } else {
      $out = $stdin | & ssh.exe -o BatchMode=yes -o StrictHostKeyChecking=accept-new "$SSH_USER@$targetHost" "bash -s" 2>&1
    }
    return ($out | Out-String)
  } catch {
    return ("ERROR: $_`n" + ($out | Out-String))
  }
}

function SnapshotNode($n) {
  $h = $n.host; $name = $n.name
  Write-Host "== SNAPSHOT $name ($h) =="

  $snap = @"
set +e
echo "=== WHOAMI ==="
whoami; id; uname -a

echo "=== SYSTEMD CAT ==="
sudo -n systemctl cat ippan-node || true

echo "=== SYSTEMD SHOW (key fields) ==="
sudo -n systemctl show ippan-node -p User -p Group -p ExecStart -p WorkingDirectory -p StateDirectory -p LogsDirectory -p RuntimeDirectory || true

echo "=== LOCAL STATUS ==="
curl -fsS http://127.0.0.1:8080/status || true

echo "=== LOCAL PEERS ==="
curl -fsS http://127.0.0.1:8080/p2p/peers || true

echo "=== JOURNAL (200) ==="
sudo -n journalctl -u ippan-node -n 200 --no-pager || true

echo "=== LISTENERS ==="
ss -lntup || true
ss -lnuap || true

echo "=== DIRS/PERMS ==="
for p in /var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config /opt/ippan /usr/local/bin ; do
  echo "== \$p ==";
  ls -ld "\$p" 2>/dev/null || true
done
echo "== pidfile =="; ls -l /var/lib/ippan/ippan-node.pid 2>/dev/null || true

echo "=== UFW ==="
sudo -n ufw status verbose || true
"@
  $out = RunRemoteScript $h $snap $false
  WriteBundle $name "00_snapshot.txt" $out
}

function ApplyFixNode($n, $allNodeIps) {
  $h = $n.host; $name = $n.name
  Write-Host "== FIX $name ($h) =="

  $node4Extra = ""
  if ($name -eq "node4" -and $MYIP -ne "") {
    $node4Extra = "sudo -n ufw allow from $MYIP/32 to any port 8080 proto tcp || true"
  }

  # Important: remove sandboxing that causes EACCES + make systemd create State/Logs/Runtime dirs.
  $fix = @"
set -e

# --- systemd override (EACCES fix) ---
sudo -n mkdir -p /etc/systemd/system/ippan-node.service.d

cat <<'EOF' | sudo -n tee /etc/systemd/system/ippan-node.service.d/override.conf >/dev/null
[Service]
User=ippan-devnet
Group=ippan-devnet

StateDirectory=ippan
LogsDirectory=ippan
RuntimeDirectory=ippan

ExecStartPre=/usr/bin/env bash -lc 'rm -f /var/lib/ippan/ippan-node.pid || true'

ProtectSystem=off
ProtectHome=off
PrivateTmp=no
NoNewPrivileges=no

ReadWritePaths=/var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config /opt/ippan

WorkingDirectory=/var/lib/ippan
EOF

sudo -n systemctl daemon-reload

# --- ensure writable dirs + ownership ---
sudo -n mkdir -p /var/lib/ippan /var/log/ippan /etc/ippan/config
sudo -n chown -R ippan-devnet:ippan-devnet /var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config || true
sudo -n chmod 0755 /var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config || true
sudo -n rm -f /var/lib/ippan/ippan-node.pid || true

# --- UFW: allow only devnet nodes + private net (no public) ---
sudo -n ufw --force enable || true
sudo -n ufw default deny incoming
sudo -n ufw default allow outgoing
sudo -n ufw allow 22/tcp || true

# best-effort delete broad rules (avoid prompts)
yes | sudo -n ufw delete allow 8080/tcp 2>/dev/null || true
yes | sudo -n ufw delete allow 9000/tcp 2>/dev/null || true
yes | sudo -n ufw delete allow 9000/udp 2>/dev/null || true

# allow inter-node ports

"@

  foreach ($ip in $allNodeIps) {
    $fix += "`n" + "sudo -n ufw allow from $ip/32 to any port 8080 proto tcp || true"
    $fix += "`n" + "sudo -n ufw allow from $ip/32 to any port 9000 proto tcp || true"
    $fix += "`n" + "sudo -n ufw allow from $ip/32 to any port 9000 proto udp || true"
  }

  $fix += @"

sudo -n ufw allow from 10.0.0.0/24 to any port 8080 proto tcp || true
sudo -n ufw allow from 10.0.0.0/24 to any port 9000 proto tcp || true
sudo -n ufw allow from 10.0.0.0/24 to any port 9000 proto udp || true

$node4Extra

sudo -n ufw status verbose || true

# --- restart + show status (bounded) ---
sudo -n systemctl restart ippan-node
sudo -n systemctl is-active ippan-node || true
sudo -n systemctl status ippan-node --no-pager -n 60 || true
"@

  $out = RunRemoteScript $h $fix $false
  WriteBundle $name "10_fix_apply.txt" $out
}

function VerifyNode($n) {
  $h = $n.host; $name = $n.name
  Write-Host "== VERIFY $name ($h) =="

  $ver = @"
set +e
echo "=== ACTIVE? ==="
sudo -n systemctl is-active ippan-node || true

echo "=== LOCAL STATUS ==="
curl -fsS http://127.0.0.1:8080/status || true

echo "=== LOCAL PEERS ==="
curl -fsS http://127.0.0.1:8080/p2p/peers || true

echo "=== JOURNAL (120) ==="
sudo -n journalctl -u ippan-node -n 120 --no-pager || true

echo "=== LISTENERS (8080/9000) ==="
ss -lntup | egrep ':(8080|9000)\b' || true
ss -lnuap | egrep ':(9000)\b' || true
"@
  $out = RunRemoteScript $h $ver $false
  WriteBundle $name "20_verify.txt" $out
}

# ---------------- RUN (NO LOOPS) ----------------
$allIps = $NODES | ForEach-Object { $_.host }

"RUN START $STAMP" | Out-File (Join-Path $BUNDLE "RUN.log") -Encoding utf8

# Snapshot before
foreach ($n in $NODES) {
  try { SnapshotNode $n } catch { Write-Host "Snapshot failed for $($n.name): $_" }
}

# Fix leaf nodes first, then bootstrap last
foreach ($n in $NODES | Where-Object { $_.name -ne "node1" }) {
  try { ApplyFixNode $n $allIps } catch { Write-Host "Fix failed for $($n.name): $_" }
}
try { ApplyFixNode ($NODES | Where-Object { $_.name -eq "node1" }) $allIps } catch { Write-Host "Fix failed for node1: $_" }

# Verify after
foreach ($n in $NODES) {
  try { VerifyNode $n } catch { Write-Host "Verify failed for $($n.name): $_" }
}

# Laptop note
$note = @()
$note += "Bundle: $BUNDLE"
$note += "Laptop check (node4):"
$note += "  curl.exe -fsS http://178.156.219.107:8080/status"
$note += "If EACCES persists, open node*/20_verify.txt and look for exact denied path in the journal."
$note | Out-File (Join-Path $BUNDLE "SUMMARY.txt") -Encoding utf8

Write-Host ""
Write-Host "DONE. Evidence bundle: $BUNDLE"

