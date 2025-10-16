# Create a simple valid transaction for IPPAN
# Using the correct Transaction struct format

Write-Host "Creating simple IPPAN transaction..." -ForegroundColor Green

# Create a minimal valid transaction
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$nonce = 1

# Create transaction with minimal required fields
$tx = @{
    id = @(0) * 32  # 32 zero bytes
    from = @(1) * 32  # 32 bytes for from address
    to = @(2) * 32    # 32 bytes for to address  
    amount = 1000
    nonce = $nonce
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

Write-Host "Transaction payload:" -ForegroundColor Yellow
Write-Host $body

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "Response body: $responseBody" -ForegroundColor Yellow
    }
}
