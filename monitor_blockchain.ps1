# Monitor blockchain activity
Write-Host "Monitoring IPPAN blockchain activity..." -ForegroundColor Green

$startTime = Get-Date
$maxWaitTime = 300 # 5 minutes

Write-Host "Starting blockchain monitoring at $startTime" -ForegroundColor Yellow
Write-Host "Will monitor for up to $maxWaitTime seconds..." -ForegroundColor Yellow

$previousHeight = -1
$previousMempool = -1

while ((Get-Date) - $startTime -lt [TimeSpan]::FromSeconds($maxWaitTime)) {
    try {
        $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
        $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
        
        $currentHeight = $state.latest_block_height
        $currentMempool = $mempool.Count
        
        if ($currentHeight -ne $previousHeight -or $currentMempool -ne $previousMempool) {
            Write-Host "`n[$(Get-Date)] Blockchain Update:" -ForegroundColor Cyan
            Write-Host "  Height: $currentHeight (was $previousHeight)" -ForegroundColor White
            Write-Host "  Mempool: $currentMempool (was $previousMempool)" -ForegroundColor White
            Write-Host "  Proposer: $($state.current_proposer)" -ForegroundColor White
            Write-Host "  Round: $($state.current_round)" -ForegroundColor White
            Write-Host "  Slot: $($state.current_slot)" -ForegroundColor White
            
            if ($currentHeight -gt $previousHeight) {
                Write-Host "  🎉 NEW BLOCK CREATED! Height increased from $previousHeight to $currentHeight" -ForegroundColor Green
            }
            
            if ($currentMempool -gt $previousMempool) {
                Write-Host "  📝 NEW TRANSACTION! Mempool increased from $previousMempool to $currentMempool" -ForegroundColor Yellow
            }
            
            $previousHeight = $currentHeight
            $previousMempool = $currentMempool
        }
        
        # Check if blockchain is building
        if ($currentHeight -gt 0) {
            Write-Host "`n🚀 BLOCKCHAIN IS BUILDING!" -ForegroundColor Green
            Write-Host "Current height: $currentHeight" -ForegroundColor Green
            Write-Host "Mempool size: $currentMempool" -ForegroundColor Green
            break
        }
        
        # Show progress every 30 seconds
        $elapsed = (Get-Date) - $startTime
        if ([int]$elapsed.TotalSeconds % 30 -eq 0) {
            Write-Host "`n[$(Get-Date)] Still monitoring... (${elapsed.TotalSeconds}s elapsed)" -ForegroundColor Gray
            Write-Host "  Height: $currentHeight, Mempool: $currentMempool" -ForegroundColor Gray
        }
    }
    catch {
        Write-Host "Error monitoring blockchain: $($_.Exception.Message)" -ForegroundColor Red
    }
    
    Start-Sleep -Seconds 5
}

$elapsed = (Get-Date) - $startTime
Write-Host "`nMonitoring completed after $($elapsed.TotalSeconds) seconds" -ForegroundColor Yellow

# Final status check
try {
    $finalState = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    $finalMempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    
    Write-Host "`nFinal Blockchain Status:" -ForegroundColor Cyan
    Write-Host "  Height: $($finalState.latest_block_height)" -ForegroundColor White
    Write-Host "  Mempool: $($finalMempool.Count)" -ForegroundColor White
    Write-Host "  Proposer: $($finalState.current_proposer)" -ForegroundColor White
    Write-Host "  Round: $($finalState.current_round)" -ForegroundColor White
    
    if ($finalState.latest_block_height -gt 0) {
        Write-Host "`n🎉 SUCCESS: Blockchain is building with height $($finalState.latest_block_height)!" -ForegroundColor Green
    } else {
        Write-Host "`n⚠️  Blockchain still at height 0. Transactions may be needed to trigger block creation." -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Error getting final status: $($_.Exception.Message)" -ForegroundColor Red
}
