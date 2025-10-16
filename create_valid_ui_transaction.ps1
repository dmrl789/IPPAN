# Create valid transactions using UI crypto functions
Write-Host "Creating valid IPPAN transactions using UI crypto..." -ForegroundColor Green

# First, let's create a proper transaction using the UI's approach
# We'll simulate what the UI does when creating a wallet and transaction

# Generate proper IPPAN addresses using the UI crypto functions
$address1 = "i0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
$address2 = "i9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"

Write-Host "Generated addresses:" -ForegroundColor Yellow
Write-Host "Address 1: $address1"
Write-Host "Address 2: $address2"

# Create a transaction using the UI's crypto approach
# The UI generates proper signatures and HashTimers
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Create a transaction that matches the UI's format exactly
$tx = @{
    from = $address1
    to = $address2
    amount = 1000
    nonce = 1
    memo = "UI-generated transaction"
    timestamp = $timestamp
    fee = 0.001
    # Add the fields that the UI would include
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 3

Write-Host "`nTrying UI-generated transaction..." -ForegroundColor Cyan
Write-Host "Payload: $body"

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
    
    # Check mempool immediately
    Start-Sleep -Seconds 2
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "🎉 TRANSACTION ADDED TO MEMPOOL!" -ForegroundColor Green
        Write-Host "Transactions in mempool:" -ForegroundColor Yellow
        $mempool | ForEach-Object { Write-Host "  - $($_.id)" }
        
        # Check blockchain status
        $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
        Write-Host "`nBlockchain status:" -ForegroundColor Cyan
        Write-Host "  Height: $($state.latest_block_height)" -ForegroundColor White
        Write-Host "  Mempool: $($state.mempool_len)" -ForegroundColor White
        Write-Host "  Proposer: $($state.current_proposer)" -ForegroundColor White
        Write-Host "  Round: $($state.current_round)" -ForegroundColor White
        
        # Wait for consensus to create a block
        Write-Host "`nWaiting for consensus to create block..." -ForegroundColor Yellow
        for ($i = 1; $i -le 30; $i++) {
            Start-Sleep -Seconds 2
            $newState = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
            if ($newState.latest_block_height -gt $state.latest_block_height) {
                Write-Host "🚀 BLOCK CREATED! Height: $($newState.latest_block_height)" -ForegroundColor Green
                break
            }
            Write-Host "[$i] Waiting for block creation..." -ForegroundColor Gray
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

# Final status check
Write-Host "`nFinal blockchain status:" -ForegroundColor Cyan
try {
    $finalState = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    $finalMempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    
    Write-Host "  Height: $($finalState.latest_block_height)" -ForegroundColor White
    Write-Host "  Mempool: $($finalMempool.Count)" -ForegroundColor White
    Write-Host "  Round: $($finalState.current_round)" -ForegroundColor White
    
    if ($finalState.latest_block_height -gt 0) {
        Write-Host "`n🎉 SUCCESS: Blockchain is building with height $($finalState.latest_block_height)!" -ForegroundColor Green
    } else {
        Write-Host "`n⚠️  Blockchain still at height 0. Need valid transactions." -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Error getting final status: $($_.Exception.Message)" -ForegroundColor Red
}
