# Use the UI wallet to create valid transactions with the provided addresses
Write-Host "Using UI wallet to create valid transactions..." -ForegroundColor Green

$address1 = "i79475a40c0cc424d96bafdd68165a34690cae60b71360a8254b11a05d7bafd7f"
$address2 = "i169af88019057a5d5c683df43abdd79843a7741981e1565966953606a65ea5f5"

Write-Host "Address 1: $address1" -ForegroundColor Yellow
Write-Host "Address 2: $address2" -ForegroundColor Yellow

# Open the UI wallet
Write-Host "`nOpening UI wallet..." -ForegroundColor Cyan
Start-Process "http://localhost:80"

Write-Host "`n🚀 IPPAN BLOCKCHAIN TRANSACTION CREATOR" -ForegroundColor Green
Write-Host "`nTo create valid transactions with these addresses:" -ForegroundColor Cyan
Write-Host "1. Go to http://localhost:80" -ForegroundColor White
Write-Host "2. Click 'Wallet' in the navigation" -ForegroundColor White
Write-Host "3. Click 'Create Wallet'" -ForegroundColor White
Write-Host "4. Generate a new wallet" -ForegroundColor White
Write-Host "5. Use the wallet to send transactions to these addresses:" -ForegroundColor White
Write-Host "   - $address1" -ForegroundColor Gray
Write-Host "   - $address2" -ForegroundColor Gray

Write-Host "`nThe UI wallet will handle all cryptographic requirements:" -ForegroundColor Yellow
Write-Host "✅ Generate proper signatures" -ForegroundColor Green
Write-Host "✅ Create valid HashTimers" -ForegroundColor Green
Write-Host "✅ Format transactions correctly" -ForegroundColor Green

Write-Host "`nWhile you create transactions in the UI, let me try one more approach..." -ForegroundColor Cyan

# Try to create a transaction using the validator's address
# Since the validator is active, maybe we can create a transaction from it
$validatorAddress = "i0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"

Write-Host "`nTrying transaction from validator to your addresses..." -ForegroundColor Yellow

# Transaction 1: Validator -> Address1
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$tx1 = @{
    from = $validatorAddress
    to = $address1
    amount = 1000
    nonce = 1
    memo = "Validator to Address1"
    timestamp = $timestamp
}

$body1 = @{
    tx = $tx1
} | ConvertTo-Json -Depth 3

try {
    $response1 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body1 -ContentType "application/json"
    Write-Host "SUCCESS: Validator -> Address1" -ForegroundColor Green
    Write-Host "Response: $($response1 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: Validator -> Address1 - $($_.Exception.Message)" -ForegroundColor Red
}

Start-Sleep -Seconds 2

# Transaction 2: Validator -> Address2
$tx2 = @{
    from = $validatorAddress
    to = $address2
    amount = 1000
    nonce = 2
    memo = "Validator to Address2"
    timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
}

$body2 = @{
    tx = $tx2
} | ConvertTo-Json -Depth 3

try {
    $response2 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body2 -ContentType "application/json"
    Write-Host "SUCCESS: Validator -> Address2" -ForegroundColor Green
    Write-Host "Response: $($response2 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: Validator -> Address2 - $($_.Exception.Message)" -ForegroundColor Red
}

# Check mempool and blockchain status
Write-Host "`nChecking blockchain status..." -ForegroundColor Cyan
try {
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "Mempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "🎉 TRANSACTIONS ADDED TO MEMPOOL!" -ForegroundColor Green
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
    Write-Host "Error checking status: $($_.Exception.Message)" -ForegroundColor Red
}

# Final status
Write-Host "`nFinal blockchain status:" -ForegroundColor Cyan
try {
    $finalState = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    Write-Host "Height: $($finalState.latest_block_height)" -ForegroundColor White
    Write-Host "Mempool: $($finalState.mempool_len)" -ForegroundColor White
    Write-Host "Round: $($finalState.current_round)" -ForegroundColor White
    Write-Host "Proposer: $($finalState.current_proposer)" -ForegroundColor White
    
    if ($finalState.latest_block_height -gt 0) {
        Write-Host "`n🎉 SUCCESS: Blockchain is building with height $($finalState.latest_block_height)!" -ForegroundColor Green
    } else {
        Write-Host "`n⚠️  Blockchain ready - use UI wallet to create valid transactions!" -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}
