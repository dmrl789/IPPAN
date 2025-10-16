# Use the UI's JavaScript functions to create valid transactions
Write-Host "Using UI JavaScript functions to create valid transactions..." -ForegroundColor Green

$address1 = "i79475a40c0cc424d96bafdd68165a34690cae60b71360a8254b11a05d7bafd7f"
$address2 = "i169af88019057a5d5c683df43abdd79843a7741981e1565966953606a65ea5f5"

Write-Host "Address 1: $address1" -ForegroundColor Yellow
Write-Host "Address 2: $address2" -ForegroundColor Yellow

# Create a PowerShell script that will use the UI's JavaScript functions
$jsScript = @"
// Use the UI's crypto functions to create valid transactions
const { generateWallet, generateAddress, signTransaction } = require('./apps/unified-ui/src/lib/crypto.ts');

async function createValidTransaction() {
    try {
        // Generate a new wallet
        const wallet = await generateWallet();
        console.log('Generated wallet:', wallet);
        
        // Create a transaction
        const transaction = {
            from: wallet.address,
            to: '$address1',
            amount: 1000,
            nonce: 1,
            memo: 'Valid transaction from UI crypto',
            timestamp: Date.now(),
            fee: 0.001
        };
        
        // Sign the transaction
        const signedTx = await signTransaction(transaction, wallet.privateKey);
        console.log('Signed transaction:', signedTx);
        
        return signedTx;
    } catch (error) {
        console.error('Error creating transaction:', error);
        return null;
    }
}

createValidTransaction();
"@

# Write the JavaScript to a file
$jsScript | Out-File -FilePath "create_valid_tx.js" -Encoding UTF8

Write-Host "`nCreated JavaScript script to use UI crypto functions..." -ForegroundColor Cyan

# Try to run the JavaScript using Node.js
Write-Host "`nTrying to run JavaScript with Node.js..." -ForegroundColor Yellow

try {
    $result = node create_valid_tx.js
    Write-Host "JavaScript result: $result" -ForegroundColor Green
}
catch {
    Write-Host "Node.js not available or error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Let me try a different approach..." -ForegroundColor Yellow
}

# Alternative approach: Create a transaction using the UI's approach
Write-Host "`nTrying alternative approach - create transaction using UI structure..." -ForegroundColor Cyan

# Create a transaction that matches the UI's expected format
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Try to create a transaction using the validator's address with all required fields
$validatorTx = @{
    from = "i0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
    to = $address1
    amount = 1000
    nonce = 1
    memo = "Validator transaction with all fields"
    timestamp = $timestamp
    fee = 0.001
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
    # Add fields that might be required for validation
    id = "tx_" + [System.Guid]::NewGuid().ToString("N")
    signature = "validator_signature_placeholder"
    hashtimer = "ht_" + [System.Guid]::NewGuid().ToString("N")
}

$validatorBody = @{
    tx = $validatorTx
} | ConvertTo-Json -Depth 3

Write-Host "`nTrying validator transaction with all fields..." -ForegroundColor Yellow
Write-Host "Payload: $validatorBody"

try {
    $validatorResponse = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $validatorBody -ContentType "application/json"
    Write-Host "SUCCESS: Validator transaction with all fields sent!" -ForegroundColor Green
    Write-Host "Response: $($validatorResponse | ConvertTo-Json)" -ForegroundColor Green
    
    # Check mempool
    Start-Sleep -Seconds 3
    $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
    Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
    
    if ($mempool.Count -gt 0) {
        Write-Host "🎉 TRANSACTION ADDED TO MEMPOOL!" -ForegroundColor Green
        Write-Host "The consensus mechanism should now create blocks!" -ForegroundColor Green
        
        # Monitor for block creation
        Write-Host "`nMonitoring for block creation..." -ForegroundColor Yellow
        for ($i = 1; $i -le 30; $i++) {
            Start-Sleep -Seconds 2
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
    Write-Host "Validator transaction failed: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "`nThe blockchain requires valid cryptographic signatures." -ForegroundColor Yellow
    Write-Host "Let me try one final approach..." -ForegroundColor Cyan
    
    # Try to create a transaction using the UI's exact format
    Write-Host "`nTrying UI exact format transaction..." -ForegroundColor Yellow
    
    $uiTx = @{
        from = $address1
        to = $address2
        amount = 1000
        nonce = 1
        memo = "UI exact format transaction"
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
    
    $uiBody = @{
        tx = $uiTx
    } | ConvertTo-Json -Depth 3
    
    try {
        $uiResponse = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $uiBody -ContentType "application/json"
        Write-Host "SUCCESS: UI exact format transaction sent!" -ForegroundColor Green
        Write-Host "Response: $($uiResponse | ConvertTo-Json)" -ForegroundColor Green
        
        # Check mempool
        Start-Sleep -Seconds 3
        $mempool = Invoke-RestMethod -Uri "http://localhost:8080/mempool" -Method GET
        Write-Host "`nMempool size: $($mempool.Count)" -ForegroundColor Green
        
        if ($mempool.Count -gt 0) {
            Write-Host "🎉 TRANSACTION ADDED TO MEMPOOL!" -ForegroundColor Green
            Write-Host "The consensus mechanism should now create blocks!" -ForegroundColor Green
            
            # Monitor for block creation
            Write-Host "`nMonitoring for block creation..." -ForegroundColor Yellow
            for ($i = 1; $i -le 30; $i++) {
                Start-Sleep -Seconds 2
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
        Write-Host "All transaction attempts failed: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "`nThe blockchain requires valid cryptographic signatures." -ForegroundColor Yellow
        Write-Host "Use the UI wallet at http://localhost:80 to create valid transactions." -ForegroundColor Cyan
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
        Write-Host "`n⚠️  Blockchain ready - use UI wallet to create valid transactions!" -ForegroundColor Yellow
        Write-Host "Go to http://localhost:80 and use the wallet functionality." -ForegroundColor Cyan
    }
}
catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}

# Clean up
Remove-Item "create_valid_tx.js" -ErrorAction SilentlyContinue
