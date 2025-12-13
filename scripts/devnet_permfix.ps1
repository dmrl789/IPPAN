# scripts/devnet_permfix.ps1
$ErrorActionPreference = "Stop"

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
$OUT = "tmp/devnet/permfix_$ts"
New-Item -ItemType Directory -Force -Path $OUT | Out-Null

$NODES = @(
  @{ name="node1"; host="188.245.97.41"  },
  @{ name="node2"; host="135.181.145.174" },
  @{ name="node3"; host="5.223.51.238"   },
  @{ name="node4"; host="178.156.219.107" }
)

function RunSsh([string]$nodeHost, [string]$cmd) {
  & ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new "ippan-devnet@$nodeHost" $cmd
}

function RunSudoBash([string]$nodeHost, [string]$bashScript) {
  $tmpLocal = Join-Path $env:TEMP ("ippan_permfix_" + [guid]::NewGuid().ToString("n") + ".sh")
  # write as UTF8 no BOM with Unix line endings
  $bashScriptUnix = $bashScript -replace "`r`n", "`n" -replace "`r", "`n"
  [System.IO.File]::WriteAllText($tmpLocal, $bashScriptUnix, (New-Object System.Text.UTF8Encoding($false)))

  $tmpRemote = "/tmp/ippan_permfix.sh"
  & scp -q -o BatchMode=yes -o StrictHostKeyChecking=accept-new $tmpLocal ("ippan-devnet@{0}:{1}" -f $nodeHost, $tmpRemote) | Out-Null
  Remove-Item -Force $tmpLocal -ErrorAction SilentlyContinue

  RunSsh $nodeHost "sudo -n bash $tmpRemote"
}

# Drop-in: remove sandbox surprises + set writable working dir
$DROPIN = @"
[Service]
User=ippan-devnet
Group=ippan-devnet
DynamicUser=no
WorkingDirectory=/var/lib/ippan

# Make sure systemd isn't blocking writes (common cause of os error 13)
ProtectSystem=off
ProtectHome=off
PrivateTmp=false

# Allow writes where node typically writes
ReadWritePaths=/var/lib/ippan /var/log/ippan /etc/ippan /opt/ippan /usr/local/bin

# Ensure dirs exist and are owned correctly
RuntimeDirectory=ippan
RuntimeDirectoryMode=0750
StateDirectory=ippan
StateDirectoryMode=0750
LogsDirectory=ippan
LogsDirectoryMode=0750
"@

$dropLocal = Join-Path $OUT "20-ippan-runtime.conf"
[System.IO.File]::WriteAllText($dropLocal, $DROPIN, (New-Object System.Text.UTF8Encoding($false)))

foreach ($n in $NODES) {
  $name = $n.name
  $nodeHost = $n.host
  $nodeOut = Join-Path $OUT $name
  New-Item -ItemType Directory -Force -Path $nodeOut | Out-Null

  Write-Host "=== [$name] SNAPSHOT (before) ==="
  try {
    RunSsh $nodeHost "sudo -n systemctl is-active ippan-node || true" | Out-File (Join-Path $nodeOut "active_before.txt")
    RunSsh $nodeHost "sudo -n systemctl cat ippan-node || true"       | Out-File (Join-Path $nodeOut "unit_before.txt")
    RunSsh $nodeHost "sudo -n journalctl -u ippan-node -n 120 --no-pager || true" | Out-File (Join-Path $nodeOut "journal_before.txt")
  } catch {
    $_ | Out-File (Join-Path $nodeOut "snapshot_before_error.txt")
  }

  Write-Host "=== [$name] APPLY FIXES ==="

  # 1) install runtime drop-in via scp + sudo install
  & scp -q -o BatchMode=yes -o StrictHostKeyChecking=accept-new $dropLocal ("ippan-devnet@{0}:/tmp/20-ippan-runtime.conf" -f $nodeHost) | Out-Null
  RunSsh $nodeHost "sudo -n mkdir -p /etc/systemd/system/ippan-node.service.d"
  RunSsh $nodeHost "sudo -n install -m 0644 /tmp/20-ippan-runtime.conf /etc/systemd/system/ippan-node.service.d/20-ippan-runtime.conf"

  # 2) fix ExecStart permission issues robustly:
  #    - find ExecStart binary path
  #    - ensure executable
  #    - copy to /usr/local/bin/ippan-node (safe exec location)
  #    - override ExecStart to use /usr/local/bin/ippan-node with same args
  $REMOTE_FIX = @'
#!/bin/bash
set -e
UNIT="ippan-node"

mkdir -p /var/lib/ippan /var/log/ippan /etc/ippan || true
chown -R ippan-devnet:ippan-devnet /var/lib/ippan /var/log/ippan || true
chmod 0750 /var/lib/ippan /var/log/ippan || true

# Extract ExecStart line
CMD=$(systemctl cat "$UNIT" | awk -F= '/^ExecStart=/{print $2; exit}')
CMD=${CMD#-}
BIN=$(printf "%s" "$CMD" | awk '{print $1}')

echo "ExecStart CMD: $CMD"
echo "ExecStart BIN: $BIN"

if [ -n "$BIN" ] && [ -e "$BIN" ]; then
  chmod 0755 "$BIN" || true
  chown root:root "$BIN" || true
  install -m 0755 "$BIN" /usr/local/bin/ippan-node
else
  # fallback: try a common path
  if [ -e "/opt/ippan/IPPAN/target/release/ippan-node" ]; then
    chmod 0755 "/opt/ippan/IPPAN/target/release/ippan-node" || true
    install -m 0755 "/opt/ippan/IPPAN/target/release/ippan-node" /usr/local/bin/ippan-node
    CMD="/opt/ippan/IPPAN/target/release/ippan-node"
  fi
fi

# Preserve args after binary token
REST=${CMD#* }
if [ "$REST" = "$CMD" ]; then REST=""; fi

mkdir -p /etc/systemd/system/ippan-node.service.d
if [ -n "$REST" ]; then
  echo "[Service]" > /etc/systemd/system/ippan-node.service.d/10-exec.conf
  echo "ExecStart=" >> /etc/systemd/system/ippan-node.service.d/10-exec.conf
  echo "ExecStart=/usr/local/bin/ippan-node $REST" >> /etc/systemd/system/ippan-node.service.d/10-exec.conf
else
  echo "[Service]" > /etc/systemd/system/ippan-node.service.d/10-exec.conf
  echo "ExecStart=" >> /etc/systemd/system/ippan-node.service.d/10-exec.conf
  echo "ExecStart=/usr/local/bin/ippan-node" >> /etc/systemd/system/ippan-node.service.d/10-exec.conf
fi

# pidfile safety
rm -f /var/lib/ippan/ippan-node.pid || true

systemctl daemon-reload
systemctl restart "$UNIT"
sleep 2
systemctl --no-pager status "$UNIT" -n 30 || true
echo "LOCAL STATUS:"
curl -fsS http://127.0.0.1:8080/status || true
echo
'@

  try {
    RunSudoBash $nodeHost $REMOTE_FIX | Out-File (Join-Path $nodeOut "apply.log")
  } catch {
    $_ | Out-File (Join-Path $nodeOut "apply_error.txt")
  }

  Write-Host "=== [$name] SNAPSHOT (after) ==="
  try {
    RunSsh $nodeHost "sudo -n systemctl is-active ippan-node || true" | Out-File (Join-Path $nodeOut "active_after.txt")
    RunSsh $nodeHost "sudo -n systemctl cat ippan-node || true"       | Out-File (Join-Path $nodeOut "unit_after.txt")
    RunSsh $nodeHost "sudo -n journalctl -u ippan-node -n 160 --no-pager || true" | Out-File (Join-Path $nodeOut "journal_after.txt")
    RunSsh $nodeHost "curl -fsS http://127.0.0.1:8080/status || true"  | Out-File (Join-Path $nodeOut "status_local.txt")
    RunSsh $nodeHost "curl -fsS http://127.0.0.1:8080/p2p/peers || true" | Out-File (Join-Path $nodeOut "peers_local.txt")
  } catch {
    $_ | Out-File (Join-Path $nodeOut "snapshot_after_error.txt")
  }
}

Write-Host "=== NODE4: dpkg lock quick fix (bounded) ==="
try {
  RunSsh "178.156.219.107" "sudo -n bash -lc 'set -e;
    systemctl stop unattended-upgrades apt-daily.service apt-daily-upgrade.service 2>/dev/null || true;
    systemctl kill --kill-who=all unattended-upgrades 2>/dev/null || true;
    for i in \$(seq 1 20); do
      if fuser /var/lib/dpkg/lock-frontend >/dev/null 2>&1; then
        echo \"lock still held (\$i/20)\"; sleep 3;
      else
        echo \"lock released\"; break;
      fi
    done
    dpkg --configure -a || true
  '" | Out-File (Join-Path $OUT "node4_dpkg_fix.txt")
} catch {
  $_ | Out-File (Join-Path $OUT "node4_dpkg_fix_error.txt")
}

Write-Host "=== FINAL VERIFY from laptop ==="
try {
  & curl.exe -fsS "http://178.156.219.107:8080/status" | Out-File (Join-Path $OUT "node4_status_from_laptop.json")
} catch {
  $_ | Out-File (Join-Path $OUT "node4_status_from_laptop_error.txt")
}

Write-Host "DONE. Evidence at: $OUT"

