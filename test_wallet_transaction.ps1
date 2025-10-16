# Test wallet transaction using the UI wallet functionality
Write-Host "Testing wallet transaction..." -ForegroundColor Green

# First, let's create a wallet using the UI crypto functions
# We'll simulate what the UI does when creating a wallet

# Generate a simple address (this is what the UI does)
$address1 = "i0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
$address2 = "i9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"

Write-Host "Generated addresses:" -ForegroundColor Yellow
Write-Host "Address 1: $address1"
Write-Host "Address 2: $address2"

# Try to create a transaction using the validator's address
# The validator address from the response was: {1, 35, 69, 103...}
# Let's try to create a transaction from the validator

$validatorAddress = "i0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

# Create a simple transaction structure
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Try the simplest possible transaction
$tx = @{
    from = $validatorAddress
    to = $address2
    amount = 100
    nonce = 1
    memo = "Test transaction"
    timestamp = $timestamp
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 3

Write-Host "`nTrying simple transaction format..." -ForegroundColor Cyan
Write-Host "Payload: $body"

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: $($response | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "Response: $responseBody" -ForegroundColor Yellow
    }
}

# Check mempool after transaction attempt
Write-Host "`nChecking mempool..." -ForegroundColor Cyan
try {
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "Mempool size: $($mempool.Count)" -ForegroundColor Green
    if ($mempool.Count -gt 0) {
        Write-Host "Transactions in mempool:" -ForegroundColor Yellow
        $mempool | ForEach-Object { Write-Host "  - $($_.id)" }
    }
}
catch {
    Write-Host "Could not check mempool: $($_.Exception.Message)" -ForegroundColor Yellow
}
