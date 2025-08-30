# IPPAN Performance Monitor
# Monitor system resources during high-TPS testing

param(
    [int]$Duration = 300,
    [string]$OutputFile = "performance_log.csv"
)

Write-Host "📊 IPPAN Performance Monitor" -ForegroundColor Green
Write-Host "Monitoring system resources for $Duration seconds..." -ForegroundColor Yellow

# Create CSV header
$csvHeader = "Timestamp,CPU_Usage,Memory_Usage,Network_IO,Disk_IO,IPPAN_Processes"
$csvHeader | Out-File -FilePath $OutputFile -Encoding UTF8

$startTime = Get-Date
$endTime = $startTime.AddSeconds($Duration)

Write-Host "Started monitoring at: $startTime" -ForegroundColor Cyan
Write-Host "Will stop at: $endTime" -ForegroundColor Cyan

while ((Get-Date) -lt $endTime) {
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    
    # Get CPU usage
    $cpu = (Get-Counter "\Processor(_Total)\% Processor Time").CounterSamples.CookedValue
    
    # Get memory usage
    $memory = (Get-Counter "\Memory\Available MBytes").CounterSamples.CookedValue
    $totalMemory = (Get-Counter "\Memory\Committed MBytes").CounterSamples.CookedValue
    
    # Get network I/O (simplified)
    $network = (Get-Counter "\Network Interface(*)\Bytes Total/sec").CounterSamples.CookedValue | Measure-Object -Sum
    $networkIO = $network.Sum
    
    # Get disk I/O (simplified)
    $disk = (Get-Counter "\PhysicalDisk(_Total)\Disk Bytes/sec").CounterSamples.CookedValue
    $diskIO = $disk
    
    # Count IPPAN processes
    $ippanProcesses = (Get-Process | Where-Object {$_.ProcessName -like "*ippan*"}).Count
    
    # Create CSV line
    $csvLine = "$timestamp,$cpu,$memory,$networkIO,$diskIO,$ippanProcesses"
    $csvLine | Out-File -FilePath $OutputFile -Append -Encoding UTF8
    
    # Display current status
    Write-Host "[$timestamp] CPU: $($cpu.ToString('F1'))% | Memory: $($memory.ToString('F0'))MB | IPPAN Processes: $ippanProcesses" -ForegroundColor White
    
    Start-Sleep -Seconds 5
}

Write-Host "`n✅ Performance monitoring completed!" -ForegroundColor Green
Write-Host "Results saved to: $OutputFile" -ForegroundColor Yellow
