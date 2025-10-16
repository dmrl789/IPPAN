# Use the UI wallet to create a valid transaction
Write-Host "Using UI wallet to create valid transaction..." -ForegroundColor Green

# Open the UI and navigate to wallet creation
Write-Host "Opening UI wallet..." -ForegroundColor Yellow
Start-Process "http://localhost:80"

Write-Host "`nPlease follow these steps in the browser:" -ForegroundColor Cyan
Write-Host "1. Go to http://localhost:80" -ForegroundColor White
Write-Host "2. Click on 'Wallet' in the navigation" -ForegroundColor White
Write-Host "3. Click on 'Create Wallet'" -ForegroundColor White
Write-Host "4. Generate a new wallet" -ForegroundColor White
Write-Host "5. Copy the generated address" -ForegroundColor White
Write-Host "6. Try to send a transaction" -ForegroundColor White

Write-Host "`nWhile you do that, let me try a different approach..." -ForegroundColor Yellow

# Let me try to create a transaction using the validator's address
# Since the validator is active, maybe we can create a transaction from it

$validatorAddress = "i0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
$recipientAddress = "i9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"

Write-Host "`nTrying transaction from validator address..." -ForegroundColor Cyan
Write-Host "Validator: $validatorAddress" -ForegroundColor Gray
Write-Host "Recipient: $recipientAddress" -ForegroundColor Gray

# Create a simple transaction
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$tx = @{
    from = $validatorAddress
    to = $recipientAddress
    amount = 100
    nonce = 1
    memo = "Validator transaction"
    timestamp = $timestamp
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 3

Write-Host "`nTransaction payload:" -ForegroundColor Yellow
Write-Host $body

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
    
    # Check if it was added to mempool
    Start-Sleep -Seconds 3
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "🎉 TRANSACTION ADDED TO MEMPOOL!" -ForegroundColor Green
        Write-Host "The consensus mechanism should now create blocks!" -ForegroundColor Green
        
        # Monitor for block creation
        Write-Host "`nMonitoring for block creation..." -ForegroundColor Yellow
        for ($i = 1; $i -le 20; $i++) {
            Start-Sleep -Seconds 3
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

Write-Host "`nFinal status:" -ForegroundColor Cyan
try {
    $finalState = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    Write-Host "Height: $($finalState.latest_block_height)" -ForegroundColor White
    Write-Host "Mempool: $($finalState.mempool_len)" -ForegroundColor White
    Write-Host "Round: $($finalState.current_round)" -ForegroundColor White
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}
