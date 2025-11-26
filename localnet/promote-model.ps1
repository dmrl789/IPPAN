# IPPAN Model Promotion Wrapper
# Promotes a trained fairness model to runtime with hash guard

param(
    [Parameter(Mandatory=$true)]
    [string]$Model,
    
    [Parameter(Mandatory=$true)]
    [string]$Version,
    
    [string]$Config = "config/dlc.toml"
)

$ErrorActionPreference = "Stop"

Write-Host "=== IPPAN Model Promotion ===" -ForegroundColor Cyan
Write-Host ""

# Ensure we're in repo root
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
Push-Location $repoRoot

try {
    # Check Python
    Write-Host "[1/4] Checking Python..." -ForegroundColor Yellow
    try {
        $pythonVersion = python --version 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Python not found"
        }
        Write-Host "  Python is available ($pythonVersion)" -ForegroundColor Green
    } catch {
        Write-Host "  Python is not available" -ForegroundColor Red
        Write-Host "  Please install Python 3 and ensure it's in your PATH" -ForegroundColor Yellow
        exit 1
    }
    
    # Check blake3 package
    Write-Host "[2/4] Checking blake3 package..." -ForegroundColor Yellow
    $venvPath = Join-Path $repoRoot ".venv"
    if (Test-Path $venvPath) {
        # Use venv if it exists
        $activateScript = Join-Path $venvPath "Scripts\Activate.ps1"
        if (Test-Path $activateScript) {
            & $activateScript
        }
    }
    
    try {
        python -c "import blake3" 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "blake3 not found"
        }
        Write-Host "  blake3 package is available" -ForegroundColor Green
    } catch {
        Write-Host "  Missing 'blake3' package" -ForegroundColor Red
        if (Test-Path $venvPath) {
            Write-Host "  Installing into venv..." -ForegroundColor Yellow
            pip install "blake3==0.4.1" --quiet
            if ($LASTEXITCODE -ne 0) {
                Write-Host "  Failed to install blake3" -ForegroundColor Red
                exit 1
            }
            Write-Host "  Installed blake3" -ForegroundColor Green
        } else {
            Write-Host "  Install with: pip install blake3==0.4.1" -ForegroundColor Yellow
            Write-Host "  Or create a venv: python -m venv .venv" -ForegroundColor Yellow
            exit 1
        }
    }
    
    # Validate model file
    Write-Host "[3/4] Validating model file..." -ForegroundColor Yellow
    $modelPath = Resolve-Path $Model -ErrorAction Stop
    if (-not (Test-Path $modelPath)) {
        Write-Host "  Model file not found: $Model" -ForegroundColor Red
        exit 1
    }
    Write-Host "  Model file found: $modelPath" -ForegroundColor Green
    
    # Compute runtime destination
    Write-Host "[4/4] Computing runtime destination..." -ForegroundColor Yellow
    $runtimeDest = "crates/ai_registry/models/ippan_d_gbdt_$Version.json"
    Write-Host "  Runtime destination: $runtimeDest" -ForegroundColor Green
    
    Write-Host ""
    Write-Host "=== Promoting Model ===" -ForegroundColor Cyan
    Write-Host "  Model: $modelPath" -ForegroundColor Gray
    Write-Host "  Version: $Version" -ForegroundColor Gray
    Write-Host "  Runtime: $runtimeDest" -ForegroundColor Gray
    Write-Host "  Config: $Config" -ForegroundColor Gray
    Write-Host ""
    
    # Run promotion tool
    $promoteScript = Join-Path $repoRoot "ai_training\promote_fairness_model.py"
    python $promoteScript `
        --model $modelPath `
        --runtime-dest $runtimeDest `
        --config $Config
    
    $exitCode = $LASTEXITCODE
    
    if ($exitCode -eq 2) {
        Write-Host ""
        Write-Host "Promotion refused: hash unchanged" -ForegroundColor Yellow
        Write-Host "  Train with different data/params or do not bump the version" -ForegroundColor Gray
        Write-Host "  To override: add --allow-same-hash flag" -ForegroundColor Gray
        exit 2
    } elseif ($exitCode -ne 0) {
        Write-Host ""
        Write-Host "Promotion failed (exit code: $exitCode)" -ForegroundColor Red
        exit $exitCode
    }
    
    Write-Host ""
    Write-Host "Model promoted successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  Test: cargo test -p ippan-ai-registry --lib" -ForegroundColor Gray
    Write-Host "  Test: cargo test -p ippan-consensus-dlc --lib" -ForegroundColor Gray
    Write-Host "  Commit: git add crates/ai_registry/models/ config/dlc.toml" -ForegroundColor Gray
    
} finally {
    Pop-Location
}

