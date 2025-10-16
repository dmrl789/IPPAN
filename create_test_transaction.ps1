# Create a test transaction using the UI wallet functionality
Write-Host "Creating test transaction using UI approach..." -ForegroundColor Green

# Let's try to create a transaction using the UI's crypto functions
# First, let's check if we can create a wallet through the UI

# Generate addresses using the same method as the UI
$address1 = "i0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
$address2 = "i9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"

Write-Host "Generated addresses:" -ForegroundColor Yellow
Write-Host "Address 1: $address1"
Write-Host "Address 2: $address2"

# Try to create a transaction using the simplest possible format
# that matches what the UI would send

$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Create a transaction that matches the UI format
$tx = @{
    from = $address1
    to = $address2
    amount = 1000
    nonce = 1
    memo = "Test transaction from UI"
    timestamp = $timestamp
    fee = 0.001
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 3

Write-Host "`nTrying UI-style transaction..." -ForegroundColor Cyan
Write-Host "Payload: $body"

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
    
    # Check if transaction was added to mempool
    Start-Sleep -Seconds 3
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "Transactions in mempool:" -ForegroundColor Yellow
        $mempool | ForEach-Object { Write-Host "  - $($_.id)" }
        
        # Check blockchain status
        $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
        Write-Host "`nBlockchain status:" -ForegroundColor Cyan
        Write-Host "  Height: $($state.latest_block_height)" -ForegroundColor White
        Write-Host "  Mempool: $($state.mempool_len)" -ForegroundColor White
        Write-Host "  Proposer: $($state.current_proposer)" -ForegroundColor White
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
