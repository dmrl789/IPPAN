# Create valid transactions using UI wallet approach
Write-Host "Creating valid transactions using UI wallet approach..." -ForegroundColor Green

$address1 = "i79475a40c0cc424d96bafdd68165a34690cae60b71360a8254b11a05d7bafd7f"
$address2 = "i169af88019057a5d5c683df43abdd79843a7741981e1565966953606a65ea5f5"

Write-Host "Address 1: $address1" -ForegroundColor Yellow
Write-Host "Address 2: $address2" -ForegroundColor Yellow

# Open the UI
Write-Host "`nOpening IPPAN UI..." -ForegroundColor Cyan
Start-Process "http://localhost:80"

# Create a transaction using the validator's address with proper structure
Write-Host "`nCreating transactions using validator approach..." -ForegroundColor Yellow

$validatorAddress = "i0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Transaction 1: Validator -> Address1
$tx1 = @{
    from = $validatorAddress
    to = $address1
    amount = 1000
    nonce = 1
    memo = "Validator to Address1"
    timestamp = $timestamp
    fee = 0.001
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    id = "tx_" + [System.Guid]::NewGuid().ToString("N")
    signature = "validator_signature_placeholder"
    hashtimer = "ht_" + [System.Guid]::NewGuid().ToString("N")
}

$body1 = @{
    tx = $tx1
} | ConvertTo-Json -Depth 3

Write-Host "`nSending Transaction 1: Validator -> Address1" -ForegroundColor Yellow
try {
    $response1 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body1 -ContentType "application/json"
    Write-Host "SUCCESS: Transaction 1 sent!" -ForegroundColor Green
    Write-Host "Response: $($response1 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: Transaction 1 - $($_.Exception.Message)" -ForegroundColor Red
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
    fee = 0.001
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    id = "tx_" + [System.Guid]::NewGuid().ToString("N")
    signature = "validator_signature_placeholder"
    hashtimer = "ht_" + [System.Guid]::NewGuid().ToString("N")
}

$body2 = @{
    tx = $tx2
} | ConvertTo-Json -Depth 3

Write-Host "`nSending Transaction 2: Validator -> Address2" -ForegroundColor Yellow
try {
    $response2 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body2 -ContentType "application/json"
    Write-Host "SUCCESS: Transaction 2 sent!" -ForegroundColor Green
    Write-Host "Response: $($response2 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: Transaction 2 - $($_.Exception.Message)" -ForegroundColor Red
}

Start-Sleep -Seconds 2

# Transaction 3: Address1 -> Address2
$tx3 = @{
    from = $address1
    to = $address2
    amount = 500
    nonce = 1
    memo = "Address1 to Address2"
    timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    fee = 0.001
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    id = "tx_" + [System.Guid]::NewGuid().ToString("N")
    signature = "address1_signature_placeholder"
    hashtimer = "ht_" + [System.Guid]::NewGuid().ToString("N")
}

$body3 = @{
    tx = $tx3
} | ConvertTo-Json -Depth 3

Write-Host "`nSending Transaction 3: Address1 -> Address2" -ForegroundColor Yellow
try {
    $response3 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body3 -ContentType "application/json"
    Write-Host "SUCCESS: Transaction 3 sent!" -ForegroundColor Green
    Write-Host "Response: $($response3 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: Transaction 3 - $($_.Exception.Message)" -ForegroundColor Red
}

Start-Sleep -Seconds 2

# Transaction 4: Address2 -> Address1
$tx4 = @{
    from = $address2
    to = $address1
    amount = 250
    nonce = 1
    memo = "Address2 to Address1"
    timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    fee = 0.001
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    id = "tx_" + [System.Guid]::NewGuid().ToString("N")
    signature = "address2_signature_placeholder"
    hashtimer = "ht_" + [System.Guid]::NewGuid().ToString("N")
}

$body4 = @{
    tx = $tx4
} | ConvertTo-Json -Depth 3

Write-Host "`nSending Transaction 4: Address2 -> Address1" -ForegroundColor Yellow
try {
    $response4 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body4 -ContentType "application/json"
    Write-Host "SUCCESS: Transaction 4 sent!" -ForegroundColor Green
    Write-Host "Response: $($response4 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: Transaction 4 - $($_.Exception.Message)" -ForegroundColor Red
}

# Check mempool status
Write-Host "`nChecking mempool status..." -ForegroundColor Cyan
try {
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "Mempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "TRANSACTIONS ADDED TO MEMPOOL!" -ForegroundColor Green
        Write-Host "The consensus mechanism should now create blocks!" -ForegroundColor Green
        
        # Monitor for block creation
        Write-Host "`nMonitoring for block creation..." -ForegroundColor Yellow
        for ($i = 1; $i -le 30; $i++) {
            Start-Sleep -Seconds 2
            $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
            Write-Host "[$i] Height: $($state.latest_block_height), Mempool: $($state.mempool_len)" -ForegroundColor White
            
            if ($state.latest_block_height -gt 0) {
                Write-Host "BLOCK CREATED! Height: $($state.latest_block_height)" -ForegroundColor Green
                break
            }
        }
    }
}
catch {
    Write-Host "Error checking mempool: $($_.Exception.Message)" -ForegroundColor Red
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
        Write-Host "`nSUCCESS: Blockchain is building with height $($finalState.latest_block_height)!" -ForegroundColor Green
    } else {
        Write-Host "`nBlockchain ready - use UI wallet to create valid transactions!" -ForegroundColor Yellow
        Write-Host "Go to http://localhost:80 and use the wallet functionality." -ForegroundColor Cyan
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}