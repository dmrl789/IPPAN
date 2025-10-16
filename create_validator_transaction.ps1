# Create a transaction from the validator address
Write-Host "Creating transaction from validator..." -ForegroundColor Green

# Convert validator address to IPPAN format
$validatorBytes = @(1, 35, 69, 103, 137, 171, 205, 239, 1, 35, 69, 103, 137, 171, 205, 239, 1, 35, 69, 103, 137, 171, 205, 239, 1, 35, 69, 103, 137, 171, 205, 239)
$validatorHex = ($validatorBytes | ForEach-Object { $_.ToString("X2") }) -join ""
$validatorAddress = "i$validatorHex"

Write-Host "Validator address: $validatorAddress" -ForegroundColor Yellow

# Create a simple transaction from validator
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$nonce = 1

# Try the simplest possible transaction format
$tx = @{
    from = $validatorAddress
    to = "i9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"
    amount = 100
    nonce = $nonce
    memo = "Test transaction from validator"
    timestamp = $timestamp
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 3

Write-Host "`nTransaction payload:" -ForegroundColor Cyan
Write-Host $body

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
    
    # Check mempool
    Start-Sleep -Seconds 2
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
    
    # Check blockchain status
    $state = Invoke-RestMethod -Uri "http://localhost:8080/state" -Method GET
    Write-Host "Block height: $($state.latest_block_height)" -ForegroundColor Green
    Write-Host "Mempool length: $($state.mempool_len)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "Response: $responseBody" -ForegroundColor Yellow
    }
}
