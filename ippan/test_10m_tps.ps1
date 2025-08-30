# IPPAN 10M TPS Test Script
# This script sets up a multi-node cluster and runs high-performance load tests

param(
    [int]$NodeCount = 8,
    [int]$TargetTPS = 10000000,
    [int]$TestDuration = 60,
    [int]$AccountCount = 10000,
    [int]$Concurrency = 2000,
    [int]$BatchSize = 1000
)

Write-Host "🚀 IPPAN 10M TPS Capability Test" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Green
Write-Host "Target TPS: $TargetTPS" -ForegroundColor Yellow
Write-Host "Test Duration: $TestDuration seconds" -ForegroundColor Yellow
Write-Host "Node Count: $NodeCount" -ForegroundColor Yellow
Write-Host "Account Count: $AccountCount" -ForegroundColor Yellow
Write-Host "Concurrency: $Concurrency" -ForegroundColor Yellow
Write-Host "Batch Size: $BatchSize" -ForegroundColor Yellow

# Step 1: Start the cluster
Write-Host "`n📡 Starting IPPAN Cluster..." -ForegroundColor Cyan
& .\run_cluster.ps1 -NodeCount $NodeCount -LogLevel "warn"

# Wait for nodes to start
Write-Host "⏳ Waiting for nodes to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 10

# Step 2: Build node URLs
$nodeUrls = @()
for ($i = 0; $i -lt $NodeCount; $i++) {
    $port = 8080 + $i
    $nodeUrls += "http://localhost:$port"
}
$nodesArg = $nodeUrls -join ","

# Step 3: Run high-performance load test
Write-Host "`n🔥 Starting 10M TPS Load Test..." -ForegroundColor Red
Write-Host "This will test the system's capability to handle $TargetTPS transactions per second" -ForegroundColor Red

cargo run --release -p ippan-loadgen-cli -- `
    --high-performance `
    --tps $TargetTPS `
    --duration $TestDuration `
    --accounts $AccountCount `
    --nodes $nodesArg `
    --concurrency $Concurrency `
    --batch-size $BatchSize `
    --output "10m_tps_results.csv"

# Step 4: Analyze results
Write-Host "`n📊 Test Complete!" -ForegroundColor Green
Write-Host "Check 10m_tps_results.csv for detailed results" -ForegroundColor Yellow

# Step 5: Stop cluster
Write-Host "`n🛑 Stopping cluster..." -ForegroundColor Cyan
Get-Process | Where-Object {$_.ProcessName -like "*ippan*"} | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Host "✅ 10M TPS test completed!" -ForegroundColor Green
