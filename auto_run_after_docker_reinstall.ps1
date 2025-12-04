$ErrorActionPreference = "Stop"

# ===== CONFIG: set your repo root here (adjust if needed) =====
$REPO = "C:\Users\yuyby\Desktop\backUp CursorSoftware\ippan nov 25 backup"

function Section($t){ Write-Host "`n=== $t ===`n" -ForegroundColor Cyan }
function Ok($t){ Write-Host "[OK] $t" -ForegroundColor Green }
function Warn($t){ Write-Host "[WARN] $t" -ForegroundColor Yellow }
function Fail($t){ throw $t }

Section "Go to repo root"
if (!(Test-Path $REPO)) { Fail "Repo path not found: $REPO" }
Set-Location $REPO

# Logs
New-Item -ItemType Directory -Force -Path .\tmp | Out-Null
$LOG = ".\tmp\auto_run_after_docker_reinstall.log"
if (Test-Path $LOG) { Remove-Item $LOG -Force }

# Helper: tee all output
Start-Transcript -Path $LOG | Out-Null

try {
  Section "Repo sanity + script presence"
  if (!(Test-Path .\scripts\post_reboot_setup.ps1)) { Fail "Missing scripts/post_reboot_setup.ps1" }
  if (!(Test-Path .\scripts\ensure_docker_stable.ps1)) { Fail "Missing scripts/ensure_docker_stable.ps1" }

  Section "Git sync (master only)"
  try {
    $currentBranch = git rev-parse --abbrev-ref HEAD 2>$null
    if ($LASTEXITCODE -ne 0) { Warn "Not in a git repository or git not available" }
    elseif ($currentBranch -ne "master") {
      Write-Host "Current branch: $currentBranch, switching to master..."
      git checkout master
      if ($LASTEXITCODE -ne 0) { Warn "Failed to checkout master (may have uncommitted changes)" }
      else { Ok "Switched to master branch" }
    } else {
      Ok "Already on master branch"
    }
    
    Write-Host "Syncing master (handles divergence)..."
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\git_sync_master.ps1
    if ($LASTEXITCODE -ne 0) { Warn "git sync failed (may have conflicts requiring manual resolution)" }
    else { Ok "Repository synced" }
  } catch {
    Warn "Git operations failed: $($_.Exception.Message)"
  }

  Section "WSL2 quick sanity"
  try { wsl -l -v } catch { Warn "WSL not available in PATH. If Docker Desktop uses WSL2 backend, enable/install WSL2." }
  try { wsl --set-default-version 2 | Out-Null } catch { }

  # --- Install Docker Desktop if missing ---
  Section "Docker Desktop install (only if missing)"
  $dockerExe = "$Env:ProgramFiles\Docker\Docker\Docker Desktop.exe"
  $dockerCli = "$Env:ProgramFiles\Docker\Docker\DockerCli.exe"

  if (!(Test-Path $dockerExe)) {
    Ok "Docker Desktop is not installed. Installing via winget..."
    $hasWinget = $false
    try { winget --version | Out-Null; $hasWinget = $true } catch { $hasWinget = $false }

    if (-not $hasWinget) {
      Fail "winget not found. Install 'App Installer' from Microsoft Store or install Docker Desktop manually, then rerun this block."
    }

    # Install (idempotent-ish). If already installed from winget, it will say so.
    winget install -e --id Docker.DockerDesktop --accept-package-agreements --accept-source-agreements

    if (!(Test-Path $dockerExe)) {
      Warn "Docker Desktop install finished but exe not found yet. If installer requested a reboot, reboot and re-run this block."
    }
  } else {
    Ok "Docker Desktop present."
  }

  # --- Start/Restart Docker service + app ---
  Section "Start Docker Desktop + force Linux engine"
  Stop-Process -Name "Docker Desktop" -Force -ErrorAction SilentlyContinue
  Stop-Process -Name "com.docker.backend" -Force -ErrorAction SilentlyContinue
  try { Restart-Service com.docker.service -ErrorAction SilentlyContinue | Out-Null } catch { }

  if (Test-Path $dockerExe) { 
    Start-Process $dockerExe | Out-Null
    Write-Host "Waiting 10 seconds for Docker Desktop to initialize..."
    Start-Sleep -Seconds 10
  } else { Fail "Docker Desktop exe still missing at $dockerExe" }

  # Force Linux engine (best effort)
  if (Test-Path $dockerCli) { & $dockerCli -SwitchLinuxEngine 2>$null | Out-Null }

  # --- Docker stability gate (YOUR script) ---
  Section "Docker stability gate (must pass before localnet)"
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\ensure_docker_stable.ps1
  Ok "Docker is stable."

  # Ensure docker context is desktop-linux (best effort)
  Section "Docker context set desktop-linux (best effort)"
  try {
    $ctx = (docker context ls 2>$null | Out-String)
    if ($ctx -match "desktop-linux") { docker context use desktop-linux 2>$null | Out-Null }
  } catch { }

  # --- Quick docker smoke test ---
  Section "Docker smoke test"
  # Add Docker to PATH if not already there
  $dockerBin = "$Env:ProgramFiles\Docker\Docker\resources\bin"
  if (Test-Path $dockerBin) {
    $env:Path = "$dockerBin;$env:Path"
  }
  docker version
  docker ps
  try { docker compose version } catch { Warn "docker compose not available; localnet may rely on it." }

  # --- Run the full pipeline (localnet → export → train → promote → gates → push) ---
  Section "Run training pipeline (post_reboot_setup.ps1)"
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\post_reboot_setup.ps1

  Ok "ALL DONE: training + promotion pipeline completed."
  Ok "Log saved at: $LOG"
}
catch {
  Warn "FAILED. See log: $LOG"
  Warn "Run diagnostics next:"
  if (Test-Path .\scripts\post_reboot_diagnostics.ps1) {
    Write-Host "powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\post_reboot_diagnostics.ps1"
  } else {
    Warn "scripts/post_reboot_diagnostics.ps1 not found."
  }
  throw
}
finally {
  Stop-Transcript | Out-Null
}

