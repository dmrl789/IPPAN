# Create transactions between the provided IPPAN addresses
Write-Host "Creating transactions between IPPAN addresses..." -ForegroundColor Green

$address1 = "i79475a40c0cc424d96bafdd68165a34690cae60b71360a8254b11a05d7bafd7f"
$address2 = "i169af88019057a5d5c683df43abdd79843a7741981e1565966953606a65ea5f5"

Write-Host "Address 1: $address1" -ForegroundColor Yellow
Write-Host "Address 2: $address2" -ForegroundColor Yellow

# Function to create a transaction
function Create-Transaction {
    param(
        [string]$From,
        [string]$To,
        [decimal]$Amount,
        [string]$Memo,
        [string]$NodeUrl
    )
    
    $timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $nonce = [System.Random]::new().Next(1, 1000000)
    
    $tx = @{
        from = $From
        to = $To
        amount = $Amount
        nonce = $nonce
        memo = $Memo
        timestamp = $timestamp
        fee = 0.001
    }
    
    $body = @{
        tx = $tx
    } | ConvertTo-Json -Depth 3
    
    try {
        $response = Invoke-RestMethod -Uri "$NodeUrl/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
        Write-Host "SUCCESS: $Memo - $($response.data.tx_hash)" -ForegroundColor Green
        return $response.data.tx_hash
    }
    catch {
        Write-Host "FAILED: $Memo - $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

Write-Host "`nCreating multiple transactions..." -ForegroundColor Cyan

# Transaction 1: Address1 -> Address2
Write-Host "`nTransaction 1: $address1 -> $address2 (1000 IPN)" -ForegroundColor Yellow
$tx1 = Create-Transaction $address1 $address2 1000.0 "Transaction 1: 1000 IPN transfer" "http://localhost:8080"

Start-Sleep -Seconds 2

# Transaction 2: Address2 -> Address1
Write-Host "`nTransaction 2: $address2 -> $address1 (500 IPN)" -ForegroundColor Yellow
$tx2 = Create-Transaction $address2 $address1 500.0 "Transaction 2: 500 IPN transfer" "http://localhost:8081"

Start-Sleep -Seconds 2

# Transaction 3: Address1 -> Address2
Write-Host "`nTransaction 3: $address1 -> $address2 (250 IPN)" -ForegroundColor Yellow
$tx3 = Create-Transaction $address1 $address2 250.0 "Transaction 3: 250 IPN transfer" "http://localhost:8080"

Start-Sleep -Seconds 2

# Transaction 4: Address2 -> Address1
Write-Host "`nTransaction 4: $address2 -> $address1 (750 IPN)" -ForegroundColor Yellow
$tx4 = Create-Transaction $address2 $address1 750.0 "Transaction 4: 750 IPN transfer" "http://localhost:8081"

Start-Sleep -Seconds 2

# Transaction 5: Address1 -> Address2
Write-Host "`nTransaction 5: $address1 -> $address2 (100 IPN)" -ForegroundColor Yellow
$tx5 = Create-Transaction $address1 $address2 100.0 "Transaction 5: 100 IPN transfer" "http://localhost:8080"

Write-Host "`nTransaction Summary:" -ForegroundColor Cyan
$transactions = @($tx1, $tx2, $tx3, $tx4, $tx5) | Where-Object { $_ -ne $null }
Write-Host "Successful transactions: $($transactions.Count)" -ForegroundColor White
foreach ($tx in $transactions) {
    Write-Host "  - $tx" -ForegroundColor Gray
}

# Check mempool status
Write-Host "`nChecking mempool status..." -ForegroundColor Cyan
try {
    $mempool1 = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "Node 1 mempool: $($mempool1.Count) transactions" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 1 mempool" -ForegroundColor Yellow
}

try {
    $mempool2 = Invoke-RestMethod -Uri "http://localhost:8081/mempool" -Method GET
    Write-Host "Node 2 mempool: $($mempool2.Count) transactions" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 2 mempool" -ForegroundColor Yellow
}

# Check blockchain status
Write-Host "`nChecking blockchain status..." -ForegroundColor Cyan
try {
    $status1 = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    Write-Host "Node 1 - Height: $($status1.latest_block_height), Mempool: $($status1.mempool_len)" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 1 status" -ForegroundColor Yellow
}

try {
    $status2 = Invoke-RestMethod -Uri "http://localhost:8081/state" -Method GET
    Write-Host "Node 2 - Height: $($status2.latest_block_height), Mempool: $($status2.mempool_len)" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 2 status" -ForegroundColor Yellow
}

# Monitor for block creation
Write-Host "`nMonitoring for block creation..." -ForegroundColor Yellow
for ($i = 1; $i -le 30; $i++) {
    Start-Sleep -Seconds 2
    try {
        $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
        Write-Host "[$i] Height: $($state.latest_block_height), Mempool: $($state.mempool_len)" -ForegroundColor White
        
        if ($state.latest_block_height -gt 0) {
            Write-Host "🚀 BLOCK CREATED! Height: $($state.latest_block_height)" -ForegroundColor Green
            break
        }
    }
    catch {
        Write-Host "Error monitoring: $($_.Exception.Message)" -ForegroundColor Red
    }
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
        Write-Host "`n⚠️  Blockchain still at height 0. Transactions may need valid signatures." -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}
