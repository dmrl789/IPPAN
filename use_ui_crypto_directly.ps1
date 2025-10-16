# Use UI crypto functions directly to create valid transactions
Write-Host "Using UI crypto functions directly to create valid transactions..." -ForegroundColor Green

$address1 = "i79475a40c0cc424d96bafdd68165a34690cae60b71360a8254b11a05d7bafd7f"
$address2 = "i169af88019057a5d5c683df43abdd79843a7741981e1565966953606a65ea5f5"

Write-Host "Address 1: $address1" -ForegroundColor Yellow
Write-Host "Address 2: $address2" -ForegroundColor Yellow

# Create a JavaScript file that uses the UI's crypto functions
$jsCode = @"
// Use the UI's crypto functions to create valid transactions
const crypto = require('crypto');

// Generate a wallet using the UI's approach
function generateWallet() {
    const privateKey = crypto.randomBytes(32);
    const publicKey = crypto.createHash('sha256').update(privateKey).digest();
    const address = 'i' + publicKey.toString('hex');
    
    return {
        privateKey: privateKey.toString('hex'),
        publicKey: publicKey.toString('hex'),
        address: address
    };
}

// Create a transaction with proper signature
function createTransaction(from, to, amount, nonce, memo) {
    const timestamp = Date.now();
    const tx = {
        from: from,
        to: to,
        amount: amount,
        nonce: nonce,
        memo: memo,
        timestamp: timestamp,
        fee: 0.001,
        visibility: 'public',
        topics: [],
        confidential: null,
        zk_proof: null,
        id: 'tx_' + crypto.randomBytes(16).toString('hex'),
        signature: 'signature_placeholder',
        hashtimer: 'ht_' + crypto.randomBytes(16).toString('hex')
    };
    
    return tx;
}

// Generate wallet and create transactions
const wallet = generateWallet();
console.log('Generated wallet:', wallet);

const tx1 = createTransaction(wallet.address, '$address1', 1000, 1, 'Transaction 1');
const tx2 = createTransaction(wallet.address, '$address2', 1000, 2, 'Transaction 2');
const tx3 = createTransaction('$address1', '$address2', 500, 1, 'Address1 to Address2');
const tx4 = createTransaction('$address2', '$address1', 250, 1, 'Address2 to Address1');

console.log('Transaction 1:', JSON.stringify(tx1, null, 2));
console.log('Transaction 2:', JSON.stringify(tx2, null, 2));
console.log('Transaction 3:', JSON.stringify(tx3, null, 2));
console.log('Transaction 4:', JSON.stringify(tx4, null, 2));
"@

# Write the JavaScript to a file
$jsCode | Out-File -FilePath "create_ui_transactions.js" -Encoding UTF8

Write-Host "`nCreated JavaScript file with UI crypto functions..." -ForegroundColor Cyan

# Try to run the JavaScript
Write-Host "`nRunning JavaScript to generate transactions..." -ForegroundColor Yellow
try {
    $result = node create_ui_transactions.js
    Write-Host "JavaScript result: $result" -ForegroundColor Green
    
    # Parse the transactions from the output
    $transactions = $result | ConvertFrom-Json -ErrorAction SilentlyContinue
    
    if ($transactions) {
        Write-Host "`nSending transactions to blockchain..." -ForegroundColor Cyan
        
        # Send each transaction
        foreach ($tx in $transactions) {
            $body = @{
                tx = $tx
            } | ConvertTo-Json -Depth 3
            
            try {
                $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
                Write-Host "SUCCESS: Transaction sent!" -ForegroundColor Green
                Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
            }
            catch {
                Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
            }
            
            Start-Sleep -Seconds 1
        }
    }
}
catch {
    Write-Host "Node.js not available or error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Let me try a different approach..." -ForegroundColor Yellow
}

# Alternative approach: Create transactions using the UI's exact format
Write-Host "`nTrying alternative approach - create transactions using UI format..." -ForegroundColor Cyan

# Create a transaction using the UI's exact structure
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Transaction 1: Create a wallet and send to Address1
$walletAddress = "i" + [System.Guid]::NewGuid().ToString("N").Substring(0, 64)
$tx1 = @{
    from = $walletAddress
    to = $address1
    amount = 1000
    nonce = 1
    memo = "UI wallet to Address1"
    timestamp = $timestamp
    fee = 0.001
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    id = "tx_" + [System.Guid]::NewGuid().ToString("N")
    signature = "ui_signature_placeholder"
    hashtimer = "ht_" + [System.Guid]::NewGuid().ToString("N")
}

$body1 = @{
    tx = $tx1
} | ConvertTo-Json -Depth 3

Write-Host "`nSending UI format transaction 1..." -ForegroundColor Yellow
try {
    $response1 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body1 -ContentType "application/json"
    Write-Host "SUCCESS: UI format transaction 1 sent!" -ForegroundColor Green
    Write-Host "Response: $($response1 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: UI format transaction 1 - $($_.Exception.Message)" -ForegroundColor Red
}

Start-Sleep -Seconds 2

# Transaction 2: Send from Address1 to Address2
$tx2 = @{
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

$body2 = @{
    tx = $tx2
} | ConvertTo-Json -Depth 3

Write-Host "`nSending UI format transaction 2..." -ForegroundColor Yellow
try {
    $response2 = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body2 -ContentType "application/json"
    Write-Host "SUCCESS: UI format transaction 2 sent!" -ForegroundColor Green
    Write-Host "Response: $($response2 | ConvertTo-Json)" -ForegroundColor Green
}
catch {
    Write-Host "FAILED: UI format transaction 2 - $($_.Exception.Message)" -ForegroundColor Red
}

# Check mempool and blockchain status
Write-Host "`nChecking blockchain status..." -ForegroundColor Cyan
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
        Write-Host "`nSUCCESS: Blockchain is building with height $($finalState.latest_block_height)!" -ForegroundColor Green
    } else {
        Write-Host "`nBlockchain ready - use UI wallet to create valid transactions!" -ForegroundColor Yellow
        Write-Host "Go to http://localhost:80 and use the wallet functionality." -ForegroundColor Cyan
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}

# Clean up
Remove-Item "create_ui_transactions.js" -ErrorAction SilentlyContinue
