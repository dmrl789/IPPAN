# IPPAN Localnet Runner for Windows
# Starts the full-stack localnet using Docker Compose

param(
    [ValidateSet("none","tiers","rotate","noise")] [string]$DriftMode = "none",
    [UInt64]$DriftSeed = 0
)

$ErrorActionPreference = "Stop"

$ComposeFile = "localnet/docker-compose.full-stack.yaml"
$ProjectName = "ippan-local"

Write-Host "=== IPPAN Localnet Runner ===" -ForegroundColor Cyan
Write-Host ""

# Step 1: Verify Docker is available
Write-Host "[1/4] Checking Docker..." -ForegroundColor Yellow
try {
    $dockerVersion = docker version --format '{{.Server.Version}}' 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Docker daemon is not running"
    }
    Write-Host "  Docker is running (version: $dockerVersion)" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Docker is not available or not running" -ForegroundColor Red
    Write-Host "  Please start Docker Desktop and ensure 'Use WSL2 based engine' is enabled" -ForegroundColor Yellow
    exit 1
}

# Step 2: Verify Docker Compose is available
Write-Host "[2/4] Checking Docker Compose..." -ForegroundColor Yellow
try {
    $composeOutput = docker compose version --short 2>&1
    $composeExitCode = $LASTEXITCODE
    if ($composeExitCode -ne 0) {
        throw "Docker Compose is not available"
    }
    Write-Host "  Docker Compose is available (version: $composeOutput)" -ForegroundColor Green
} catch {
    Write-Host "  Docker Compose is not available" -ForegroundColor Red
    Write-Host "  Please ensure Docker Desktop includes the Compose plugin" -ForegroundColor Yellow
    exit 1
}

# Step 3: Check compose file exists
Write-Host "[3/4] Checking compose file..." -ForegroundColor Yellow
if (-not (Test-Path $ComposeFile)) {
    Write-Host "  Compose file not found: $ComposeFile" -ForegroundColor Red
    exit 1
}
Write-Host "  Compose file found" -ForegroundColor Green

# Step 4: Configure metrics drift (if enabled)
if ($DriftMode -ne "none") {
    $env:IPPAN_STATUS_METRICS_DRIFT = "1"
    $env:IPPAN_STATUS_METRICS_DRIFT_MODE = $DriftMode
    $env:IPPAN_STATUS_METRICS_DRIFT_SEED = "$DriftSeed"
    Write-Host "  Metrics drift enabled: mode=$DriftMode seed=$DriftSeed" -ForegroundColor Cyan
} else {
    $env:IPPAN_STATUS_METRICS_DRIFT = "0"
    $env:IPPAN_STATUS_METRICS_DRIFT_MODE = "none"
    $env:IPPAN_STATUS_METRICS_DRIFT_SEED = "0"
}

# Step 5: Start the stack
Write-Host "[5/5] Starting localnet services..." -ForegroundColor Yellow
Write-Host "  This may take a few minutes on first run (building images)..." -ForegroundColor Gray

try {
    docker compose -f $ComposeFile -p $ProjectName up -d --remove-orphans
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to start services"
    }
    Write-Host "  Services started successfully" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed to start services" -ForegroundColor Red
    Write-Host "  Check logs with: docker compose -f $ComposeFile -p $ProjectName logs" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "=== Localnet Status ===" -ForegroundColor Cyan
docker compose -f $ComposeFile -p $ProjectName ps

Write-Host ""
Write-Host "=== Endpoints ===" -ForegroundColor Cyan
Write-Host "  Node RPC:        http://localhost:8080" -ForegroundColor White
Write-Host "  Gateway API:     http://localhost:8081/api" -ForegroundColor White
Write-Host "  Unified UI:     http://localhost:3000" -ForegroundColor White
Write-Host ""

Write-Host "=== Quick Commands ===" -ForegroundColor Cyan
Write-Host "  View logs:       docker compose -f $ComposeFile -p $ProjectName logs -f" -ForegroundColor Gray
Write-Host "  Check health:    curl http://localhost:8080/health" -ForegroundColor Gray
Write-Host "  Check status:   Invoke-WebRequest -Uri http://localhost:8080/status -UseBasicParsing" -ForegroundColor Gray
Write-Host "  Stop localnet:   .\localnet\stop.ps1" -ForegroundColor Gray
Write-Host ""

Write-Host "=== Verification ===" -ForegroundColor Cyan
Write-Host "  Verify /status: status_schema_version=2 and metrics_available=true" -ForegroundColor Yellow
Write-Host ""

Write-Host "Localnet is running!" -ForegroundColor Green
