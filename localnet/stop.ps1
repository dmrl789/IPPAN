# IPPAN Localnet Stopper for Windows
# Stops all localnet services and removes containers/volumes

$ErrorActionPreference = "Stop"

$ComposeFile = "localnet/docker-compose.full-stack.yaml"
$ProjectName = "ippan-local"

Write-Host "=== Stopping IPPAN Localnet ===" -ForegroundColor Cyan
Write-Host ""

# Check if services are running
$running = docker compose -f $ComposeFile -p $ProjectName ps --format json 2>&1 | ConvertFrom-Json
$hasRunning = $running | Where-Object { $_.State -eq "running" -or $_.State -eq "restarting" }

if (-not $hasRunning) {
    Write-Host "  No running services found" -ForegroundColor Yellow
    Write-Host "  Cleaning up any remaining containers/volumes..." -ForegroundColor Gray
} else {
    Write-Host "  Stopping services..." -ForegroundColor Yellow
}

try {
    docker compose -f $ComposeFile -p $ProjectName down -v --remove-orphans
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to stop services"
    }
    Write-Host "  ✓ Localnet stopped and cleaned up" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Error stopping services" -ForegroundColor Red
    Write-Host "  You may need to manually stop containers:" -ForegroundColor Yellow
    Write-Host "    docker compose -f $ComposeFile -p $ProjectName down -v" -ForegroundColor Gray
    exit 1
}

Write-Host ""
Write-Host "✓ Localnet stopped successfully" -ForegroundColor Green
