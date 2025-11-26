# IPPAN Localnet Dataset Exporter Wrapper
# Calls the Python exporter script in RPC mode

$ErrorActionPreference = "Stop"

$RpcUrl = "http://localhost:8080"
$Samples = 120
$Interval = 5
$OutputPath = "ai_training/localnet_training.csv"

Write-Host "=== IPPAN Localnet Dataset Exporter ===" -ForegroundColor Cyan
Write-Host ""

# Check if Python is available
Write-Host "[1/3] Checking Python..." -ForegroundColor Yellow
try {
    $pythonVersion = python --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Python not found"
    }
    Write-Host "  Python is available ($pythonVersion)" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Python is not available" -ForegroundColor Red
    Write-Host "  Please install Python 3 and ensure it's in your PATH" -ForegroundColor Yellow
    exit 1
}

# Check if requests library is available
Write-Host "[2/3] Checking Python dependencies..." -ForegroundColor Yellow
try {
    python -c "import requests" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "requests library not found"
    }
    Write-Host "  Python dependencies OK" -ForegroundColor Green
} catch {
    Write-Host "  Missing 'requests' library" -ForegroundColor Red
    Write-Host "  Install with: pip install requests" -ForegroundColor Yellow
    exit 1
}

# Check if RPC endpoint is reachable
Write-Host "[3/3] Checking RPC endpoint..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$RpcUrl/health" -UseBasicParsing -TimeoutSec 5 -ErrorAction Stop
    if ($response.StatusCode -eq 200) {
        Write-Host "  RPC endpoint is reachable" -ForegroundColor Green
    } else {
        throw "RPC endpoint returned status $($response.StatusCode)"
    }
} catch {
    Write-Host "  ✗ RPC endpoint is not reachable" -ForegroundColor Red
    Write-Host "  Ensure localnet is running: .\localnet\run.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "=== Exporting Dataset ===" -ForegroundColor Cyan
Write-Host "  RPC URL: $RpcUrl" -ForegroundColor Gray
Write-Host "  Samples: $Samples" -ForegroundColor Gray
Write-Host "  Interval: ${Interval}s" -ForegroundColor Gray
Write-Host "  Output: $OutputPath" -ForegroundColor Gray
Write-Host ""

# Run the Python exporter
$scriptPath = Join-Path $PSScriptRoot "..\ai_training\export_localnet_dataset.py"
$scriptPath = Resolve-Path $scriptPath -ErrorAction Stop

python $scriptPath `
    --mode rpc `
    --rpc $RpcUrl `
    --samples $Samples `
    --interval $Interval `
    --out $OutputPath

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Export failed" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Dataset exported successfully to $OutputPath" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  Train model: python ai_training\train_ippan_d_gbdt.py" -ForegroundColor Gray

