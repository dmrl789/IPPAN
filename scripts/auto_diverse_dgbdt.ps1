# scripts/auto_diverse_dgbdt.ps1

$ErrorActionPreference = "Stop"

function Write-Section($t) { Write-Host "`n=== $t ===`n" -ForegroundColor Cyan }
function Fail($m) { throw $m }
function Require-File($p) { if (-not (Test-Path $p)) { Fail "Missing required file: $p (run from repo root?)" } }

function WslPath([string]$winPath) {
  $p = (Resolve-Path $winPath).Path
  return (wsl.exe wslpath -a "$p").Trim()
}
function Run-WSL([string]$wslRoot, [string]$cmd) {
  & wsl.exe -e bash -lc "cd '$wslRoot' && $cmd"
  if ($LASTEXITCODE -ne 0) { Fail "WSL command failed: $cmd" }
}
function Try-WSL([string]$wslRoot, [string]$cmd) {
  & wsl.exe -e bash -lc "cd '$wslRoot' && $cmd"
  return $LASTEXITCODE
}

function Start-Localnet([string]$repoRoot) {
  Write-Section "Starting localnet (with log capture)"
  $helper = Join-Path $repoRoot "wait_for_docker_and_start.ps1"
  $runner = Join-Path $repoRoot "localnet\run.ps1"
  $logDir = Join-Path $repoRoot "tmp"
  New-Item -ItemType Directory -Force -Path $logDir | Out-Null
  $log = Join-Path $logDir "localnet_start.log"
  if (Test-Path $log) { Remove-Item $log -Force }

  # Clean up any existing containers first (force remove)
  Write-Host "Cleaning up existing containers..."
  try {
    $ErrorActionPreference = "SilentlyContinue"
    docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local down --remove-orphans 2>&1 | Out-Null
    # Force remove any remaining containers
    docker rm -f $(docker ps -aq --filter "name=ippan-") 2>&1 | Out-Null
    $ErrorActionPreference = "Stop"
  } catch {
    # Ignore cleanup errors
  }

  Require-File $runner
  Write-Host "Using runner: $runner"
  $raw = Get-Content $runner -Raw

  $runnerArgs = @("-DriftMode","tiers")
  if ($raw -match '(?i)\bNodeCount\b') { $runnerArgs += @("-NodeCount","8") }
  elseif ($raw -match '(?i)\bNodes\b') { $runnerArgs += @("-Nodes","8") }
  elseif ($raw -match '(?i)\bValidators\b') { $runnerArgs += @("-Validators","8") }

  # Start localnet in background
  Write-Host "Starting localnet in background..."
  $allArgs = @("-NoProfile","-ExecutionPolicy","Bypass","-File",$runner) + $runnerArgs
  $proc = Start-Process powershell.exe -PassThru -ArgumentList $allArgs -WindowStyle Hidden -WorkingDirectory $repoRoot
  
  # Wait for containers to start, then capture logs
  Write-Host "Waiting for containers to start (up to 60 seconds)..."
  for ($i=1; $i -le 30; $i++) {
    Start-Sleep -Seconds 2
    try {
      $ErrorActionPreference = "SilentlyContinue"
      $containers = docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local ps -q 2>&1 | Where-Object { $_ -notmatch "level=warning" -and $_ -notmatch "obsolete" }
      $ErrorActionPreference = "Stop"
      if ($containers -and ($containers | Measure-Object).Count -gt 0) {
        Write-Host "Containers detected, capturing logs..."
        break
      }
    } catch {
      # Continue waiting
    }
  }
  
  try {
    docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local logs --tail=100 2>&1 | Out-File -FilePath $log -Encoding UTF8
  } catch {
    "Localnet starting... (logs will be available once containers are running)" | Out-File -FilePath $log -Encoding UTF8
  }

  return @{ Proc = $proc; Log = $log }
}

function Get-StatusJson {
  try { return Invoke-RestMethod -Uri "http://127.0.0.1:8080/status" -TimeoutSec 3 } catch { return $null }
}

function Wait-ForStatusHealthy([string]$logPath) {
  Write-Section "Waiting for /status (metrics_available=true, validators>=2)"
  Write-Host "This may take 3-5 minutes for containers to build and start..." -ForegroundColor Yellow
  
  # First wait for containers to be running
  Write-Host "Waiting for containers to be running..."
  for ($i=1; $i -le 180; $i++) {
    try {
      $ErrorActionPreference = "SilentlyContinue"
      $running = docker compose -f localnet/docker-compose.full-stack.yaml -p ippan-local ps --format json 2>&1 | ConvertFrom-Json | Where-Object { $_.State -eq "running" }
      $ErrorActionPreference = "Stop"
      if ($running -and ($running | Measure-Object).Count -ge 3) {
        Write-Host "Containers are running, waiting for /status endpoint..."
        break
      }
    } catch {
      # Continue waiting
    }
    if ($i % 30 -eq 0) {
      Write-Host "  Still waiting for containers... ($i/180 seconds)" -ForegroundColor Gray
    }
    Start-Sleep -Seconds 1
  }
  
  # Now wait for /status to become healthy
  for ($i=1; $i -le 900; $i++) {
    $j = Get-StatusJson
    if ($null -ne $j) {
      $cons = $j.consensus
      $ma = $cons.metrics_available
      $vals = $cons.validators
      $count = 0
      if ($null -ne $vals) { $count = $vals.Count }
      if ($ma -eq $true -and $count -ge 2) {
        Write-Host "OK: metrics_available=true, validators=$count" -ForegroundColor Green
        return
      }
      if ($i % 30 -eq 0) {
        Write-Host "  Waiting for status... metrics_available=$ma, validators=$count ($i/900 seconds)" -ForegroundColor Gray
      }
    } else {
      if ($i % 30 -eq 0) {
        Write-Host "  Waiting for /status endpoint... ($i/900 seconds)" -ForegroundColor Gray
      }
    }
    Start-Sleep -Seconds 1
  }

  Write-Host "`n--- localnet_start.log (tail) ---"
  if (Test-Path $logPath) { Get-Content $logPath -Tail 200 }
  Fail "/status did not become healthy. See localnet_start.log tail above."
}

function Export-Dataset([string]$wslRoot, [string]$outWsl) {
  # Use 2000 samples with 0.1s interval to ensure >=2000 rows and multiple rounds
  $rc = Try-WSL $wslRoot "python ai_training/export_localnet_dataset.py --rpc http://127.0.0.1:8080 --samples 2000 --interval 0.1 --out '$outWsl'"
  if ($rc -eq 0) { return }
  $rc = Try-WSL $wslRoot "python ai_training/export_localnet_dataset.py --rpc-url http://127.0.0.1:8080 --samples 2000 --interval 0.1 --output '$outWsl'"
  if ($rc -eq 0) { return }
  # Fallback with defaults
  Run-WSL $wslRoot "python ai_training/export_localnet_dataset.py --rpc http://127.0.0.1:8080 --out '$outWsl'"
}

function QA-Dataset([string]$wslRoot, [string]$csvWsl) {
  Run-WSL $wslRoot @"
python - <<'PY'

import pandas as pd

p='$csvWsl'

df=pd.read_csv(p)

required=["timestamp_utc","round_id","validator_id","uptime_ratio_7d","validated_blocks_7d","missed_blocks_7d","avg_latency_ms","slashing_events_90d","stake_normalized","peer_reports_quality","fairness_score_scaled"]

missing=[c for c in required if c not in df.columns]

if missing: raise SystemExit(f"FAIL: missing columns: {missing}")



rows=len(df)

uv=df["validator_id"].nunique()

ur=df["round_id"].nunique()

print("rows:", rows)

print("unique validator_id:", uv)

print("unique round_id:", ur)

if rows < 2000: raise SystemExit("FAIL: too few rows (<2000)")

if uv < 2: raise SystemExit("FAIL: need >=2 unique validator_id")

if ur < 2: raise SystemExit("FAIL: need >=2 unique round_id")



# Ensure at least some variance

for c in ["uptime_ratio_7d","validated_blocks_7d","missed_blocks_7d","avg_latency_ms","peer_reports_quality","fairness_score_scaled"]:

    nun=int(df[c].nunique(dropna=True))

    if nun < 5:

        print(f"WARNING: low variation in {c} (unique={nun})")



print("OK: dataset diversity checks passed")

PY

"@

}

function Train-Model([string]$wslRoot, [string]$csvWsl, [string]$modelWsl) {
  $rc = Try-WSL $wslRoot "python ai_training/train_ippan_d_gbdt.py --csv '$csvWsl' --out '$modelWsl'"
  if ($rc -eq 0) { return }
  $rc = Try-WSL $wslRoot "python ai_training/train_ippan_d_gbdt.py --input '$csvWsl' --output '$modelWsl'"
  if ($rc -eq 0) { return }
  Run-WSL $wslRoot "python ai_training/train_ippan_d_gbdt.py --in '$csvWsl' --out '$modelWsl'"
}

function Update-DlcToml-PathHash([string]$repoRoot, [string]$wslRoot, [string]$modelRelUnix) {
  Write-Section "Updating config/dlc.toml (path + hash)"
  $tomlPath = Join-Path $repoRoot "config\dlc.toml"
  Require-File $tomlPath
  $toml = Get-Content $tomlPath -Raw

  $pathKey = ([regex]::Match($toml, '(?mi)^\s*([a-z0-9_]*model[a-z0-9_]*path)\s*=\s*".*"\s*$')).Groups[1].Value
  $hashKey = ([regex]::Match($toml, '(?mi)^\s*([a-z0-9_]*model[a-z0-9_]*hash)\s*=\s*"[0-9a-f]{64}"\s*$')).Groups[1].Value
  if ([string]::IsNullOrWhiteSpace($pathKey)) { Fail "Could not find model path key in config/dlc.toml" }
  if ([string]::IsNullOrWhiteSpace($hashKey)) { Fail "Could not find model hash key in config/dlc.toml" }

  $zero = ("0" * 64)
  $toml2 = [regex]::Replace($toml, "(?mi)^\s*$pathKey\s*=\s*`".*`"\s*$", "$pathKey = `"$modelRelUnix`"")
  $toml2 = [regex]::Replace($toml2, "(?mi)^\s*$hashKey\s*=\s*`"[0-9a-f]{64}`"\s*$", "$hashKey = `"$zero`"")
  Set-Content -Path $tomlPath -Value $toml2 -Encoding UTF8

  $outFile = Join-Path $repoRoot "tmp\verify_model_hash.out"
  if (Test-Path $outFile) { Remove-Item $outFile -Force }

  $cmd = "cargo run -p ippan-ai-core --bin verify_model_hash -- config/dlc.toml"
  & wsl.exe -e bash -lc "cd '$wslRoot' && $cmd" *> $outFile

  $txt = Get-Content $outFile -Raw
  # Extract hash from error message: "computed {hash}" or success: "expected value {hash}"
  $computed = $null
  if ($txt -match '(?i)computed\s+([0-9a-f]{64})') {
    $computed = $matches[1].ToLowerInvariant()
  } elseif ($txt -match '(?i)expected value\s+([0-9a-f]{64})') {
    $computed = $matches[1].ToLowerInvariant()
  } else {
    # Fallback: find any 64-char hex that's not zeros
    $hashes = [regex]::Matches($txt, '(?i)\b[0-9a-f]{64}\b') | ForEach-Object { $_.Value.ToLowerInvariant() } | Select-Object -Unique
    $computed = $hashes | Where-Object { $_ -ne $zero } | Select-Object -Last 1
  }
  if (-not $computed) { Fail "Could not extract computed model hash from verify_model_hash output.`n$txt" }

  $toml3 = Get-Content $tomlPath -Raw
  $toml3 = [regex]::Replace($toml3, "(?mi)^\s*$hashKey\s*=\s*`"[0-9a-f]{64}`"\s*$", "$hashKey = `"$computed`"")
  Set-Content -Path $tomlPath -Value $toml3 -Encoding UTF8

  Run-WSL $wslRoot $cmd
}

function Run-Gates([string]$wslRoot) {
  Write-Section "Running gates"
  Run-WSL $wslRoot "cargo test -p ippan-consensus-dlc --test fairness_invariants -- --nocapture"
  Run-WSL $wslRoot "cargo test -p ippan-ai-registry --doc"
  Run-WSL $wslRoot "cargo test -p ippan-rpc"
}

function Commit-And-Push([string]$wslRoot, [string]$modelRelUnix) {
  Write-Section "Commit + push"
  Run-WSL $wslRoot "git status --porcelain"
  Run-WSL $wslRoot "git ls-files 'ai_assets/datasets/**' || true"

  Run-WSL $wslRoot "git add config/dlc.toml '$modelRelUnix'"
  Run-WSL $wslRoot "git commit -m 'chore(ai): train and promote diverse localnet D-GBDT model (auto after reboot)'"
  Run-WSL $wslRoot "git push origin master"
}

# ---------------- MAIN ----------------

Write-Section "Auto Diverse D-GBDT (after reboot)"

Require-File "config/dlc.toml"
Require-File "localnet/run.ps1"

$repoRootWin = (Get-Location).Path
$repoRootWsl = WslPath $repoRootWin

# Docker stable gate
Write-Section "Docker stable gate"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRootWin "scripts\ensure_docker_stable.ps1")

# Sync master (use Windows script for divergence handling)
Write-Section "Sync master (handles divergence)"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repoRootWin "scripts\git_sync_master.ps1")

# Ensure ignore exists
if (-not (Test-Path ".gitignore")) { Set-Content -Path ".gitignore" -Value "" }
if (-not (Select-String -Path ".gitignore" -Pattern "^ai_assets/datasets/" -Quiet)) {
  Add-Content -Path ".gitignore" -Value "`n# Local training datasets (do not commit)`nai_assets/datasets/`n"
}

# Start localnet
$local = Start-Localnet $repoRootWin
Write-Host "Localnet PID=$($local.Proc.Id) log=$($local.Log)"

Wait-ForStatusHealthy $local.Log

# Export + QA loop (needs >=2 validators and >=2 rounds)
Write-Section "Export dataset (retry until diverse)"
New-Item -ItemType Directory -Force -Path "ai_assets/datasets/localnet" | Out-Null
$ok = $false
$csvWin = ""

for ($attempt=1; $attempt -le 15; $attempt++) {
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
  $csvWin = Join-Path $repoRootWin "ai_assets\datasets\localnet\localnet_diverse_${stamp}_a${attempt}.csv"
  $csvWsl = WslPath $csvWin

  Write-Host "Export attempt $attempt -> $csvWin"
  Export-Dataset $repoRootWsl $csvWsl

  try {
    QA-Dataset $repoRootWsl $csvWsl
    $ok = $true
    break
  } catch {
    Write-Host "QA failed (attempt $attempt): $($_.Exception.Message)" -ForegroundColor Yellow
    Start-Sleep -Seconds 4
  }
}

if (-not $ok) {
  Write-Host "`n--- localnet_start.log (tail) ---"
  if (Test-Path $local.Log) { Get-Content $local.Log -Tail 200 }
  Fail "Could not produce a dataset meeting diversity thresholds after retries."
}

Write-Host "Using dataset: $csvWin" -ForegroundColor Green

# Train
Write-Section "Train model"
$stamp2 = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$modelRelUnix = "crates/ai_registry/models/ippan_d_gbdt_localnet_diverse_${stamp2}.json"
$modelWin = Join-Path $repoRootWin ($modelRelUnix -replace "/", "\")
$modelWsl = WslPath $modelWin

Train-Model $repoRootWsl (WslPath $csvWin) $modelWsl
Run-WSL $repoRootWsl "test -f '$modelRelUnix' && ls -lah '$modelRelUnix'"

# Promote
Update-DlcToml-PathHash $repoRootWin $repoRootWsl $modelRelUnix

# Gates
Run-Gates $repoRootWsl

# Commit + push
Commit-And-Push $repoRootWsl $modelRelUnix

Write-Section "DONE"
Write-Host "Dataset (local, ignored): $csvWin"
Write-Host "Model (tracked): $modelRelUnix"
