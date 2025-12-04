# scripts/post_reboot_setup.ps1
# Post-reboot setup: stabilize Docker, clean containers, start localnet, run auto D-GBDT workflow

$ErrorActionPreference = "Stop"

# Add Docker to PATH if not already there
$dockerBin = "$Env:ProgramFiles\Docker\Docker\resources\bin"
if (Test-Path $dockerBin) {
  $env:Path = "$dockerBin;$env:Path"
}

function Section($t){ Write-Host "`n=== $t ===`n" -ForegroundColor Cyan }
function Ok($t){ Write-Host "[OK] $t" -ForegroundColor Green }
function Warn($t){ Write-Host "[WARN] $t" -ForegroundColor Yellow }
function Fail($t){ throw $t }

# --- sanity: repo root expected files ---
Section "Repo sanity"
if (!(Test-Path .\scripts\auto_diverse_dgbdt.ps1)) { Fail "Missing scripts/auto_diverse_dgbdt.ps1 (run from repo root?)" }
if (!(Test-Path .\scripts\ensure_docker_stable.ps1)) { Fail "Missing scripts/ensure_docker_stable.ps1" }
if (!(Test-Path .\localnet\run.ps1)) { Fail "Missing localnet/run.ps1" }
New-Item -ItemType Directory -Force -Path .\tmp | Out-Null

# --- Git: ensure on master and up to date ---
Section "Git: checkout master and pull"
try {
  $currentBranch = git rev-parse --abbrev-ref HEAD 2>$null
  if ($LASTEXITCODE -ne 0) { Fail "Not in a git repository" }
  if ($currentBranch -ne "master") {
    Write-Host "Current branch: $currentBranch, switching to master..."
    git checkout master
    if ($LASTEXITCODE -ne 0) { Fail "Failed to checkout master" }
  }
  Ok "On master branch"
  
  Write-Host "Syncing master (handles divergence)..."
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\git_sync_master.ps1
  if ($LASTEXITCODE -ne 0) { Warn "git sync failed (may have conflicts requiring manual resolution)" }
  else { Ok "Repository synced" }
} catch {
  Warn "Git operations failed: $($_.Exception.Message)"
}

# --- WSL sanity ---
Section "WSL sanity"
try { wsl -l -v } catch { Warn "wsl command failed; install/enable WSL2 first." }

# Force default WSL version 2 (harmless if already set)
try { wsl --set-default-version 2 | Out-Null } catch { }

# --- Docker: force Linux engine + context, then demand stability ---
Section "Docker: force Linux engine + stable gate"

$dockerCli = "$Env:ProgramFiles\Docker\Docker\DockerCli.exe"
$dockerDesktop = "$Env:ProgramFiles\Docker\Docker\Docker Desktop.exe"

# Restart Docker Desktop stack (clean slate after reboot)
Stop-Process -Name "Docker Desktop" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "com.docker.backend" -Force -ErrorAction SilentlyContinue
try { Restart-Service com.docker.service -ErrorAction SilentlyContinue | Out-Null } catch { }

# Relaunch Docker Desktop
if (Test-Path $dockerDesktop) { Start-Process $dockerDesktop | Out-Null } else { Warn "Docker Desktop exe not found at expected path." }

# Force Linux engine mode (best-effort)
if (Test-Path $dockerCli) { & $dockerCli -SwitchLinuxEngine 2>$null | Out-Null }

# Ensure docker context is desktop-linux when available
try {
  $ctx = (docker context ls 2>$null | Out-String)
  if ($ctx -match "desktop-linux") { docker context use desktop-linux 2>$null | Out-Null }
} catch { }

# Use your existing stability script (it already retries + restarts)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\ensure_docker_stable.ps1
Ok "Docker stable gate passed"

# --- Clean up broken/stale localnet containers (targeted; no system prune) ---
Section "Clean stale localnet containers (targeted)"
try { docker ps -a --format "table {{.Names}}\t{{.Status}}\t{{.Image}}" } catch { Warn "docker ps failed even after stability gate; stopping."; throw }

# Look for likely localnet containers by name patterns (safe; adjust patterns if needed)
$names = docker ps -a --format "{{.Names}}" | Where-Object { $_ -match "(?i)ippan|localnet|findag|rpc|node|validator" } | Select-Object -Unique
if ($names.Count -gt 0) {
  Write-Host "Removing containers:"; $names | ForEach-Object { " - $_" }
  foreach ($n in $names) { docker rm -f $n 2>$null | Out-Null }
  Ok "Removed matching stale containers"
} else {
  Ok "No obvious stale localnet containers found"
}

# Remove orphan networks created by previous runs (safe-ish)
try { docker network prune -f | Out-Null; Ok "Pruned unused docker networks" } catch { Warn "docker network prune failed (non-fatal)" }

# --- Start localnet now (log capture) ---
Section "Start localnet (log capture)"
$log = ".\tmp\localnet_start_boot.log"
if (Test-Path $log) { Remove-Item $log -Force }

# Prefer your helper if it exists
$errLog = $log -replace '\.log$', '_stderr.log'
if (Test-Path .\wait_for_docker_and_start.ps1) {
  $p = Start-Process powershell.exe -PassThru -ArgumentList @("-NoProfile","-ExecutionPolicy","Bypass","-File",".\wait_for_docker_and_start.ps1") -RedirectStandardOutput $log -RedirectStandardError $errLog
} else {
  $p = Start-Process powershell.exe -PassThru -ArgumentList @("-NoProfile","-ExecutionPolicy","Bypass","-File",".\localnet\run.ps1","-DriftMode","tiers") -RedirectStandardOutput $log -RedirectStandardError $errLog
}
# Merge stderr into main log
if (Test-Path $errLog) {
  Add-Content -Path $log -Value "`n=== STDERR ==="
  Get-Content $errLog | Add-Content -Path $log
  Remove-Item $errLog -Force
}

Write-Host "Localnet PID=$($p.Id) log=$log"

# --- Health check loop: require /status healthy (no manual waiting) ---
Section "Wait for /status healthy (metrics + >=2 validators)"
$ok = $false
for ($i=1; $i -le 600; $i++) {
  try {
    $j = Invoke-RestMethod -Uri "http://127.0.0.1:8080/status" -TimeoutSec 2
    $ma = $j.consensus.metrics_available
    $vc = 0
    if ($null -ne $j.consensus.validators) { $vc = $j.consensus.validators.Count }
    if ($ma -eq $true -and $vc -ge 2) { $ok = $true; break }
  } catch { }
  Start-Sleep -Seconds 1
}

if (-not $ok) {
  Warn "/status not healthy. Showing localnet log tail:"
  if (Test-Path $log) { Get-Content $log -Tail 250 }
  Warn "Also showing docker ps -a:"
  docker ps -a
  Fail "Localnet did not become healthy; fix Docker/localnet errors shown above."
}
Ok "/status healthy (metrics + multi-validator)"

# --- Run the full automated pipeline script and capture output ---
Section "Run auto diverse D-GBDT workflow"
$autoLog = ".\tmp\auto_diverse_dgbdt_run.log"
if (Test-Path $autoLog) { Remove-Item $autoLog -Force }

# Run and tee to log
$workflowExitCode = 0
try {
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\auto_diverse_dgbdt.ps1 *>&1 | Tee-Object -FilePath $autoLog
  $workflowExitCode = $LASTEXITCODE
} catch {
  $workflowExitCode = 1
  Write-Host "Workflow script threw exception: $($_.Exception.Message)" -ForegroundColor Red
}

if ($workflowExitCode -ne 0) {
  Warn "Workflow completed with errors (exit code: $workflowExitCode)"
  Write-Host "`nInspect logs:"
  Write-Host " - Localnet: $log"
  Write-Host " - Workflow: $autoLog"
  Write-Host "`nRun diagnostics: .\scripts\post_reboot_diagnostics.ps1"
  Fail "Post-reboot setup failed. See logs above."
}

Ok "Post-reboot setup completed successfully!"
Write-Host "`nLogs available at:"
Write-Host " - Localnet: $log"
Write-Host " - Workflow: $autoLog"

