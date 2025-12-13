# scripts/devnet_one_shot_fix.ps1

# One-shot devnet repair: fixes systemd EACCES + tight but correct UFW allowlists + bounded verify.

# No Docker. No password changes. No infinite loops.



$ErrorActionPreference = "Stop"



function NowStamp { (Get-Date).ToString("yyyyMMdd_HHmmss") }



$STAMP = NowStamp

$BUNDLE = Join-Path "tmp/devnet" ("one_shot_fix_" + $STAMP)

New-Item -ItemType Directory -Force -Path $BUNDLE | Out-Null



$SSH_USER = "ippan-devnet"



$NODES = @(

  @{ name="node1"; host="188.245.97.41"; role="bootstrap"; private="10.0.0.2" },

  @{ name="node2"; host="135.181.145.174"; role="validator"; private="10.0.0.3" },

  @{ name="node3"; host="5.223.51.238";  role="validator"; private="" },

  @{ name="node4"; host="178.156.219.107"; role="observer"; private="" }

)



# Your current public IP for node4 RPC allowlist (best-effort; if fails, we don't block script)

$MYIP = ""

try { $MYIP = (curl.exe -fsS https://api.ipify.org).Trim() } catch { $MYIP = "" }



function WriteBundle($nodeName, $fileName, $content) {

  $dir = Join-Path $BUNDLE $nodeName

  New-Item -ItemType Directory -Force -Path $dir | Out-Null

  $path = Join-Path $dir $fileName

  $content | Out-File -FilePath $path -Encoding utf8

}



function Ssh($hostname, $cmd) {

  # Use bash -lc to ensure consistent quoting on Ubuntu

  $escaped = $cmd.Replace('`','``')

  $full = "bash -lc " + ('"' + $escaped.Replace('"','\"') + '"')

  & ssh.exe -o BatchMode=yes -o StrictHostKeyChecking=accept-new "$SSH_USER@$hostname" $full 2>&1

}



function TrySsh($hostname, $cmd) {

  try { return Ssh $hostname $cmd } catch { return ($_ | Out-String) }

}



function GatherSnapshot($n) {

  $h = $n.host

  $name = $n.name



  Write-Host "== SNAPSHOT $name ($h) =="

  WriteBundle $name "00_whoami.txt" (TrySsh $h "whoami; id; uname -a")

  WriteBundle $name "01_systemd_cat.txt" (TrySsh $h "sudo -n systemctl cat ippan-node || true")

  WriteBundle $name "02_systemd_show.txt" (TrySsh $h "sudo -n systemctl show ippan-node -p User -p Group -p ExecStart -p WorkingDirectory -p StateDirectory -p LogsDirectory -p RuntimeDirectory || true")

  WriteBundle $name "03_status.txt" (TrySsh $h "curl -fsS http://127.0.0.1:8080/status || true")

  WriteBundle $name "04_peers.txt" (TrySsh $h "curl -fsS http://127.0.0.1:8080/p2p/peers || true")

  WriteBundle $name "05_journal_200.txt" (TrySsh $h "sudo -n journalctl -u ippan-node -n 200 --no-pager || true")

  WriteBundle $name "06_ps_ports.txt" (TrySsh $h "ss -lntup || true; ss -lnuap || true")

  WriteBundle $name "07_dirs_perms.txt" (TrySsh $h @'

set -e

for p in /var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config /opt/ippan /usr/local/bin ; do

  echo "== $p ==";

  ls -ld "$p" 2>/dev/null || true

done

echo "== pidfile =="

ls -l /var/lib/ippan/ippan-node.pid 2>/dev/null || true

'@)

  WriteBundle $name "08_ufw.txt" (TrySsh $h "sudo -n ufw status verbose || true")

}



function ApplyFix($n) {

  $h = $n.host

  $name = $n.name



  Write-Host "== FIX $name ($h) =="



  # 1) systemd drop-in: make writable dirs explicit + remove restrictive sandboxing + remove stale pid

  $override = @'

set -e

sudo -n mkdir -p /etc/systemd/system/ippan-node.service.d



cat <<'EOF' | sudo -n tee /etc/systemd/system/ippan-node.service.d/override.conf >/dev/null

[Service]

User=ippan-devnet

Group=ippan-devnet



# Make systemd create dirs with correct ownership (prevents EACCES)

StateDirectory=ippan

LogsDirectory=ippan

RuntimeDirectory=ippan



# Ensure pidfile never blocks startup

ExecStartPre=/usr/bin/env bash -lc 'rm -f /var/lib/ippan/ippan-node.pid || true'



# Remove common sandboxing that causes os error 13 during early bootstrap

ProtectSystem=off

ProtectHome=off

PrivateTmp=no

NoNewPrivileges=no



# Ensure these paths are writable even if upstream unit tightens them later

ReadWritePaths=/var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config /opt/ippan



WorkingDirectory=/var/lib/ippan

EOF



sudo -n systemctl daemon-reload

'@



  WriteBundle $name "10_apply_override.log" (TrySsh $h $override)



  # 2) Ownership sanity (idempotent)

  $perms = @'

set -e

sudo -n mkdir -p /var/lib/ippan /var/log/ippan /etc/ippan/config

sudo -n chown -R ippan-devnet:ippan-devnet /var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config || true

sudo -n chmod 0755 /var/lib/ippan /var/log/ippan /etc/ippan /etc/ippan/config || true

sudo -n rm -f /var/lib/ippan/ippan-node.pid || true

'@

  WriteBundle $name "11_fix_perms.log" (TrySsh $h $perms)



  # 3) UFW allowlist between nodes (no public exposure)

  # Allow inter-node 8080 and 9000 only from devnet nodes + 10.0.0.0/24 (private net)

  $ips = $NODES | ForEach-Object { $_.host }

  $allowRules = "set -e; sudo -n ufw --force enable || true; sudo -n ufw default deny incoming; sudo -n ufw default allow outgoing; sudo -n ufw allow 22/tcp || true;"



  foreach ($ip in $ips) {

    $allowRules += " sudo -n ufw allow from $ip/32 to any port 8080 proto tcp || true;"

    $allowRules += " sudo -n ufw allow from $ip/32 to any port 9000 proto tcp || true;"

    $allowRules += " sudo -n ufw allow from $ip/32 to any port 9000 proto udp || true;"

  }



  $allowRules += " sudo -n ufw allow from 10.0.0.0/24 to any port 8080 proto tcp || true;"

  $allowRules += " sudo -n ufw allow from 10.0.0.0/24 to any port 9000 proto tcp || true;"

  $allowRules += " sudo -n ufw allow from 10.0.0.0/24 to any port 9000 proto udp || true;"



  # On node4, also allow your laptop IP to 8080 (keep remote status access)

  if ($name -eq "node4" -and $MYIP -ne "") {

    $allowRules += " sudo -n ufw allow from $MYIP/32 to any port 8080 proto tcp || true;"

  }



  # Remove broad public allow if present (best-effort)

  $allowRules += " sudo -n ufw delete allow 8080/tcp 2>/dev/null || true;"

  $allowRules += " sudo -n ufw delete allow 9000/tcp 2>/dev/null || true;"

  $allowRules += " sudo -n ufw delete allow 9000/udp 2>/dev/null || true;"

  $allowRules += " sudo -n ufw status verbose || true;"



  WriteBundle $name "12_ufw_fix.log" (TrySsh $h $allowRules)



  # 4) Restart service (bounded)

  WriteBundle $name "13_restart.log" (TrySsh $h "sudo -n systemctl restart ippan-node; sudo -n systemctl is-active ippan-node || true; sudo -n systemctl status ippan-node --no-pager -n 30 || true")

}



function Verify($n) {

  $h = $n.host

  $name = $n.name

  Write-Host "== VERIFY $name ($h) =="



  WriteBundle $name "20_status_after.txt" (TrySsh $h "curl -fsS http://127.0.0.1:8080/status || true")

  WriteBundle $name "21_peers_after.txt" (TrySsh $h "curl -fsS http://127.0.0.1:8080/p2p/peers || true")

  WriteBundle $name "22_journal_after_120.txt" (TrySsh $h "sudo -n journalctl -u ippan-node -n 120 --no-pager || true")

}



# ---------- EXECUTION (NO LOOPS) ----------

"RUN START $STAMP" | Out-File (Join-Path $BUNDLE "RUN.log") -Encoding utf8



# A) Snapshot before

foreach ($n in $NODES) { GatherSnapshot $n }



# B) Apply fixes to leaf nodes first (2/3/4), then bootstrap last (1)

$leafs = $NODES | Where-Object { $_.name -ne "node1" }

foreach ($n in $leafs) { ApplyFix $n }

ApplyFix ($NODES | Where-Object { $_.name -eq "node1" })



# C) Verify after

foreach ($n in $NODES) { Verify $n }



# D) Final: laptop-facing check reminder

$finalNote = @()

$finalNote += "Bundle: $BUNDLE"

$finalNote += "Laptop check:"

$finalNote += "  curl.exe -fsS http://178.156.219.107:8080/status"

$finalNote += "If any node still shows Permission denied (os error 13), see: */22_journal_after_120.txt"

$finalNote | Out-File (Join-Path $BUNDLE "SUMMARY.txt") -Encoding utf8



Write-Host ""

Write-Host "DONE. Evidence bundle: $BUNDLE"

Write-Host "Next: open SUMMARY.txt and node journals if still failing."

