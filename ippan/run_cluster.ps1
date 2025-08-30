# IPPAN Multi-Node Cluster Script
# Run multiple nodes for high-TPS testing

param(
    [int]$NodeCount = 4,
    [int]$StartPort = 8080,
    [string]$LogLevel = "info"
)

Write-Host "🚀 Starting IPPAN Cluster with $NodeCount nodes..." -ForegroundColor Green

# Kill any existing ippan processes
Write-Host "🔄 Stopping existing nodes..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*ippan*"} | Stop-Process -Force -ErrorAction SilentlyContinue

# Start nodes
$jobs = @()
for ($i = 0; $i -lt $NodeCount; $i++) {
    $httpPort = $StartPort + $i
    $p2pPort = $StartPort + 100 + $i
    
    Write-Host "Starting node $($i+1) on HTTP:$httpPort, P2P:$p2pPort" -ForegroundColor Cyan
    
    $job = Start-Job -ScriptBlock {
        param($port, $p2pPort, $logLevel, $nodeId)
        Set-Location $using:PWD
        cargo run --release -p ippan-node -- --http-port $port --p2p-port $p2pPort --log-level $logLevel
    } -ArgumentList $httpPort, $p2pPort, $LogLevel, $i
    
    $jobs += $job
    
    # Wait a bit between starts
    Start-Sleep -Seconds 2
}

Write-Host "✅ All $NodeCount nodes started!" -ForegroundColor Green
Write-Host "📊 Node URLs:" -ForegroundColor Yellow
for ($i = 0; $i -lt $NodeCount; $i++) {
    $httpPort = $StartPort + $i
    Write-Host "  Node $($i+1): http://localhost:$httpPort" -ForegroundColor White
}

Write-Host "`n🛑 Press Ctrl+C to stop all nodes" -ForegroundColor Red

try {
    # Keep script running
    while ($true) {
        Start-Sleep -Seconds 1
    }
}
finally {
    Write-Host "`n🔄 Stopping all nodes..." -ForegroundColor Yellow
    $jobs | Stop-Job
    $jobs | Remove-Job
    Get-Process | Where-Object {$_.ProcessName -like "*ippan*"} | Stop-Process -Force -ErrorAction SilentlyContinue
    Write-Host "✅ All nodes stopped" -ForegroundColor Green
}
