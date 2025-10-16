# Create a proper IPPAN transaction with all required fields
Write-Host "Creating proper IPPAN transaction..." -ForegroundColor Green

# Create a transaction that matches the exact Transaction struct
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Create the transaction with all required fields
$tx = @{
    id = @(0) * 32  # 32 zero bytes for ID
    from = @(1) * 32  # 32 bytes for from address
    to = @(2) * 32    # 32 bytes for to address
    amount = 1000
    nonce = 1
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    signature = @(0) * 64  # 64 zero bytes for signature
    hashtimer = @{
        time_prefix = @(0) * 7  # 7 bytes
        hash_suffix = @(0) * 25  # 25 bytes
    }
    timestamp = @{
        "0" = $timestamp
    }
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 5

Write-Host "`nTrying proper transaction format..." -ForegroundColor Cyan
Write-Host "Payload: $body"

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
    
    # Check mempool
    Start-Sleep -Seconds 3
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "🎉 TRANSACTION ADDED TO MEMPOOL!" -ForegroundColor Green
        Write-Host "Consensus should now create blocks!" -ForegroundColor Green
        
        # Monitor for block creation
        Write-Host "`nMonitoring for block creation..." -ForegroundColor Yellow
        for ($i = 1; $i -le 30; $i++) {
            Start-Sleep -Seconds 2
            $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
            Write-Host "[$i] Height: $($state.latest_block_height), Mempool: $($state.mempool_len)" -ForegroundColor White
            
            if ($state.latest_block_height -gt 0) {
                Write-Host "🚀 BLOCK CREATED! Height: $($state.latest_block_height)" -ForegroundColor Green
                break
            }
        }
    }
}
catch {
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "Response: $responseBody" -ForegroundColor Yellow
    }
}

# Final status
Write-Host "`nFinal blockchain status:" -ForegroundColor Cyan
try {
    $finalState = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    $finalMempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    
    Write-Host "Height: $($finalState.latest_block_height)" -ForegroundColor White
    Write-Host "Mempool: $($finalMempool.Count)" -ForegroundColor White
    Write-Host "Round: $($finalState.current_round)" -ForegroundColor White
    
    if ($finalState.latest_block_height -gt 0) {
        Write-Host "`n🎉 SUCCESS: Blockchain is building!" -ForegroundColor Green
    } else {
        Write-Host "`n⚠️  Still at height 0. Need valid transactions." -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}
