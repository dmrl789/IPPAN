# Simple blockchain monitoring
Write-Host "Monitoring IPPAN blockchain..." -ForegroundColor Green

for ($i = 1; $i -le 60; $i++) {
    try {
        $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
        $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
        
        Write-Host "[$i] Height: $($state.latest_block_height), Mempool: $($mempool.Count), Round: $($state.current_round)" -ForegroundColor White
        
        if ($state.latest_block_height -gt 0) {
            Write-Host "SUCCESS: Blockchain is building! Height: $($state.latest_block_height)" -ForegroundColor Green
            break
        }
    }
    catch {
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    }
    
    Start-Sleep -Seconds 5
}

Write-Host "Monitoring complete." -ForegroundColor Yellow
