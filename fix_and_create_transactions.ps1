# Fix transaction validation and create valid transactions
Write-Host "Fixing IPPAN transaction validation and creating valid transactions..." -ForegroundColor Green

$address1 = "i79475a40c0cc424d96bafdd68165a34690cae60b71360a8254b11a05d7bafd7f"
$address2 = "i169af88019057a5d5c683df43abdd79843a7741981e1565966953606a65ea5f5"

Write-Host "Address 1: $address1" -ForegroundColor Yellow
Write-Host "Address 2: $address2" -ForegroundColor Yellow

# Let me try a different approach - create a transaction using the validator's private key
# Since the validator is active, maybe we can create a transaction from it

Write-Host "`nTrying to create transactions using validator approach..." -ForegroundColor Cyan

# First, let me check if we can create a transaction using the validator's address
$validatorAddress = "i0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"

# Create a transaction with proper structure
$timestamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()

# Try to create a transaction using the validator's address
$tx = @{
    from = $validatorAddress
    to = $address1
    amount = 1000
    nonce = 1
    memo = "Validator transaction"
    timestamp = $timestamp
    # Add fields that might be required
    visibility = "public"
    topics = @()
    confidential = $null
    zk_proof = $null
}

$body = @{
    tx = $tx
} | ConvertTo-Json -Depth 3

Write-Host "`nTrying validator transaction..." -ForegroundColor Yellow
Write-Host "Payload: $body"

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $body -ContentType "application/json"
    Write-Host "SUCCESS: Validator transaction sent!" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Green
    
    # Check if it was added to mempool
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
    Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Let me try a different approach..." -ForegroundColor Yellow
    
    # Try to create a transaction using the UI's approach
    Write-Host "`nTrying UI-style transaction..." -ForegroundColor Cyan
    
    # Create a transaction that matches the UI format
    $uiTx = @{
        from = $address1
        to = $address2
        amount = 1000
        nonce = 1
        memo = "UI-style transaction"
        timestamp = $timestamp
        fee = 0.001
        # Add UI-specific fields
        visibility = "public"
        topics = @()
        confidential = $null
        zk_proof = $null
    }
    
    $uiBody = @{
        tx = $uiTx
    } | ConvertTo-Json -Depth 3
    
    try {
        $uiResponse = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $uiBody -ContentType "application/json"
        Write-Host "SUCCESS: UI-style transaction sent!" -ForegroundColor Green
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
        Write-Host "UI-style transaction also failed: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "`nThe issue is that transactions need valid cryptographic signatures." -ForegroundColor Yellow
        Write-Host "Let me try one more approach..." -ForegroundColor Cyan
        
        # Try to create a transaction using the validator's address with proper structure
        Write-Host "`nTrying validator transaction with proper structure..." -ForegroundColor Yellow
        
        $validatorTx = @{
            from = $validatorAddress
            to = $address1
            amount = 1000
            nonce = 1
            memo = "Validator transaction with proper structure"
            timestamp = $timestamp
            fee = 0.001
            visibility = "public"
            topics = @()
            confidential = $null
            zk_proof = $null
        }
        
        $validatorBody = @{
            tx = $validatorTx
        } | ConvertTo-Json -Depth 3
        
        try {
            $validatorResponse = Invoke-RestMethod -Uri "http://localhost:8080/api/v1/transaction" -Method POST -Body $validatorBody -ContentType "application/json"
            Write-Host "SUCCESS: Validator transaction with proper structure sent!" -ForegroundColor Green
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
            Write-Host "All transaction attempts failed: $($_.Exception.Message)" -ForegroundColor Red
            Write-Host "`nThe blockchain requires valid cryptographic signatures." -ForegroundColor Yellow
            Write-Host "Use the UI wallet at http://localhost:80 to create valid transactions." -ForegroundColor Cyan
        }
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
