# IPPAN Transaction Script
# Sends multiple transactions between nodes to build the blockchain

Write-Host "Starting IPPAN Blockchain Transactions..." -ForegroundColor Green

# Generate some test addresses
$address1 = "i0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
$address2 = "i9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"
$address3 = "i1111111111111111111111111111111111111111111111111111111111111111"

Write-Host "Generated test addresses:" -ForegroundColor Yellow
Write-Host "Address 1: $address1"
Write-Host "Address 2: $address2" 
Write-Host "Address 3: $address3"

# Function to send a transaction
function Send-Transaction {
    param(
        [string]$From,
        [string]$To,
        [decimal]$Amount,
        [string]$NodeUrl
    )
    
    $timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $tx = @{
        from = $From
        to = $To
        amount = $Amount
        fee = 0.001
        memo = "Test transaction from $From to $To"
        timestamp = $timestamp
    }
    
    $body = @{
        tx = $tx
    } | ConvertTo-Json -Depth 3
    
    try {
        $response = Invoke-RestMethod -Uri "$NodeUrl/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
        Write-Host "Transaction sent: $($response.data.tx_hash)" -ForegroundColor Green
        return $response.data.tx_hash
    }
    catch {
        Write-Host "Transaction failed: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# Function to check node status
function Check-NodeStatus {
    param([string]$NodeUrl, [string]$NodeName)
    
    try {
        $status = Invoke-RestMethod -Uri "$NodeUrl/health" -Method GET
        Write-Host "$NodeName is healthy - Uptime: $($status.uptime_ms)ms, Requests: $($status.req_count)" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "$NodeName is not responding" -ForegroundColor Red
        return $false
    }
}

# Check both nodes are running
Write-Host "`nChecking node status..." -ForegroundColor Cyan
$node1Healthy = Check-NodeStatus "http://localhost:8080" "Node 1"
$node2Healthy = Check-NodeStatus "http://localhost:8081" "Node 2"

if (-not $node1Healthy -or -not $node2Healthy) {
    Write-Host "Nodes are not ready. Please start the nodes first." -ForegroundColor Red
    exit 1
}

Write-Host "`nStarting transaction sequence..." -ForegroundColor Cyan

# Send multiple transactions to build the blockchain
$transactions = @()

# Transaction 1: Address1 -> Address2
Write-Host "`nTransaction 1: $address1 -> $address2 (100 IPN)" -ForegroundColor Yellow
$tx1 = Send-Transaction $address1 $address2 100.0 "http://localhost:8080"
if ($tx1) { $transactions += $tx1 }

Start-Sleep -Seconds 2

# Transaction 2: Address2 -> Address3  
Write-Host "`nTransaction 2: $address2 -> $address3 (50 IPN)" -ForegroundColor Yellow
$tx2 = Send-Transaction $address2 $address3 50.0 "http://localhost:8081"
if ($tx2) { $transactions += $tx2 }

Start-Sleep -Seconds 2

# Transaction 3: Address3 -> Address1
Write-Host "`nTransaction 3: $address3 -> $address1 (25 IPN)" -ForegroundColor Yellow
$tx3 = Send-Transaction $address3 $address1 25.0 "http://localhost:8080"
if ($tx3) { $transactions += $tx3 }

Start-Sleep -Seconds 2

# Transaction 4: Address1 -> Address3
Write-Host "`nTransaction 4: $address1 -> $address3 (75 IPN)" -ForegroundColor Yellow
$tx4 = Send-Transaction $address1 $address3 75.0 "http://localhost:8081"
if ($tx4) { $transactions += $tx4 }

Start-Sleep -Seconds 2

# Transaction 5: Address2 -> Address1
Write-Host "`nTransaction 5: $address2 -> $address1 (30 IPN)" -ForegroundColor Yellow
$tx5 = Send-Transaction $address2 $address1 30.0 "http://localhost:8080"
if ($tx5) { $transactions += $tx5 }

Write-Host "`nTransaction Summary:" -ForegroundColor Cyan
Write-Host "Total transactions sent: $($transactions.Count)" -ForegroundColor White
foreach ($tx in $transactions) {
    Write-Host "  - $tx" -ForegroundColor Gray
}

# Check mempool status
Write-Host "`nChecking mempool status..." -ForegroundColor Cyan

try {
    $mempool1 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/mempool" -Method GET
    Write-Host "Node 1 mempool: $($mempool1.transactions.Count) transactions" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 1 mempool" -ForegroundColor Yellow
}

try {
    $mempool2 = Invoke-RestMethod -Uri "http://localhost:8081/api/v1/mempool" -Method GET  
    Write-Host "Node 2 mempool: $($mempool2.transactions.Count) transactions" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 2 mempool" -ForegroundColor Yellow
}

# Check blockchain status
Write-Host "`nChecking blockchain status..." -ForegroundColor Cyan

try {
    $status1 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/status" -Method GET
    Write-Host "Node 1 - Height: $($status1.consensus.latest_height), Slot: $($status1.consensus.current_slot)" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 1 status" -ForegroundColor Yellow
}

try {
    $status2 = Invoke-RestMethod -Uri "http://localhost:8081/api/v1/status" -Method GET
    Write-Host "Node 2 - Height: $($status2.consensus.latest_height), Slot: $($status2.consensus.current_slot)" -ForegroundColor Green
}
catch {
    Write-Host "Could not check Node 2 status" -ForegroundColor Yellow
}

Write-Host "`nTransaction sequence completed! Blockchain is building..." -ForegroundColor Green
Write-Host "Check the UI at http://localhost:80 to see the transactions" -ForegroundColor Cyan
