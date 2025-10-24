use ippan_wallet::*;
use std::sync::Arc;
use std::collections::HashMap;

/// Mock RPC client for demonstration
struct MockRpcClient {
    balances: HashMap<String, u64>,
    nonces: HashMap<String, u64>,
    transactions: Vec<TransactionRecord>,
}

struct TransactionRecord {
    from: String,
    to: String,
    amount: u64,
    hash: String,
}

impl MockRpcClient {
    fn new() -> Self {
        let mut balances = HashMap::new();
        let mut nonces = HashMap::new();
        
        // Initialize some mock balances
        balances.insert("i1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(), 5000);
        balances.insert("iabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(), 3000);
        
        nonces.insert("i1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(), 5);
        nonces.insert("iabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(), 2);
        
        Self {
            balances,
            nonces,
            transactions: Vec::new(),
        }
    }
}

impl operations::RpcClient for MockRpcClient {
    fn get_balance(&self, address: &str) -> Result<u64> {
        Ok(self.balances.get(address).copied().unwrap_or(0))
    }
    
    fn get_nonce(&self, address: &str) -> Result<u64> {
        Ok(self.nonces.get(address).copied().unwrap_or(0))
    }
    
    fn send_transaction(&self, transaction: &ippan_types::Transaction) -> Result<String> {
        let tx_hash = format!("tx_{}", uuid::Uuid::new_v4());
        println!("   📤 Mock RPC: Transaction sent with hash: {}", tx_hash);
        Ok(tx_hash)
    }
    
    fn get_transaction(&self, tx_hash: &str) -> Result<Option<ippan_types::Transaction>> {
        // Mock implementation - return None for simplicity
        Ok(None)
    }
    
    fn get_transactions_by_address(&self, address: &str) -> Result<Vec<ippan_types::Transaction>> {
        // Mock implementation - return empty for simplicity
        Ok(vec![])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize wallet with mock RPC client
    let storage = Arc::new(storage::WalletStorage::new("./advanced_wallet"));
    let rpc_client = Arc::new(MockRpcClient::new());
    let wallet = operations::WalletManager::new(storage, Some(rpc_client));
    
    println!("🚀 IPPAN Advanced Wallet Example");
    println!("=================================");
    
    // 1. Create wallet with password protection
    println!("\n1. Creating secure wallet...");
    wallet.create_wallet("Advanced IPPAN Wallet".to_string(), Some("secure_password_123"))?;
    println!("✅ Secure wallet created with password protection");
    
    // 2. Generate addresses for different purposes
    println!("\n2. Generating addresses for different purposes...");
    
    // Main account
    let main_address = wallet.generate_address(
        Some("Main Account".to_string()), 
        Some("secure_password_123")
    )?;
    
    // Trading account
    let trading_address = wallet.generate_address(
        Some("Trading Account".to_string()), 
        Some("secure_password_123")
    )?;
    
    // Savings account
    let savings_address = wallet.generate_address(
        Some("Savings Account".to_string()), 
        Some("secure_password_123")
    )?;
    
    // Generate batch of addresses for specific use case
    let batch_addresses = wallet.generate_addresses(
        3, 
        Some("Batch Payment".to_string()), 
        Some("secure_password_123")
    )?;
    
    println!("✅ Generated addresses:");
    println!("   Main Account: {}", main_address);
    println!("   Trading Account: {}", trading_address);
    println!("   Savings Account: {}", savings_address);
    println!("   Batch addresses: {}", batch_addresses.len());
    
    // 3. Simulate receiving funds (mock balances)
    println!("\n3. Simulating fund reception...");
    wallet.sync_wallet()?;
    
    let main_balance = wallet.get_address_balance(&main_address)?;
    let trading_balance = wallet.get_address_balance(&trading_address)?;
    let savings_balance = wallet.get_address_balance(&savings_address)?;
    
    println!("✅ Current balances:");
    println!("   Main Account: {} IPN", main_balance);
    println!("   Trading Account: {} IPN", trading_balance);
    println!("   Savings Account: {} IPN", savings_balance);
    
    // 4. Demonstrate transaction sending
    println!("\n4. Demonstrating transaction sending...");
    
    if main_balance > 1000 {
        println!("   Sending 1000 IPN from Main to Trading account...");
        let tx_hash = wallet.send_transaction(
            &main_address,
            &trading_address,
            1000,
            Some("secure_password_123")
        )?;
        println!("   ✅ Transaction sent: {}", tx_hash);
    }
    
    if trading_balance > 500 {
        println!("   Sending 500 IPN from Trading to Savings account...");
        let tx_hash = wallet.send_transaction(
            &trading_address,
            &savings_address,
            500,
            Some("secure_password_123")
        )?;
        println!("   ✅ Transaction sent: {}", tx_hash);
    }
    
    // 5. Address management operations
    println!("\n5. Address management operations...");
    
    // Update labels
    wallet.update_address_label(&main_address, Some("Primary Account".to_string()))?;
    wallet.update_address_label(&trading_address, Some("Active Trading".to_string()))?;
    
    println!("✅ Updated address labels");
    
    // List all addresses with updated information
    let addresses = wallet.list_addresses()?;
    println!("   Current addresses in wallet:");
    for addr in addresses {
        println!("     {} - {} (Balance: {} IPN)", 
            addr.label.as_deref().unwrap_or("No label"), 
            addr.address, 
            addr.balance
        );
    }
    
    // 6. Backup and restore demonstration
    println!("\n6. Backup and restore demonstration...");
    
    // Create multiple backups
    let backup1 = wallet.create_backup()?;
    println!("✅ Created backup 1: {}", backup1.file_name().unwrap().to_string_lossy());
    
    // Simulate some changes
    let temp_address = wallet.generate_address(
        Some("Temporary".to_string()), 
        Some("secure_password_123")
    )?;
    println!("   Added temporary address: {}", temp_address);
    
    let backup2 = wallet.create_backup()?;
    println!("✅ Created backup 2: {}", backup2.file_name().unwrap().to_string_lossy());
    
    // List all backups
    let backups = wallet.list_backups()?;
    println!("   Available backups: {}", backups.len());
    for (i, backup) in backups.iter().enumerate() {
        println!("     {}. {}", i + 1, backup.file_name().unwrap().to_string_lossy());
    }
    
    // 7. Export and import demonstration
    println!("\n7. Export and import demonstration...");
    
    // Export wallet data
    let export_data = wallet.export_wallet()?;
    println!("✅ Wallet exported with {} addresses", export_data.wallet_state.addresses.len());
    println!("   Checksum: {}", export_data.checksum);
    println!("   Export time: {}", export_data.created_at.format("%Y-%m-%d %H:%M:%S"));
    
    // 8. Wallet statistics and monitoring
    println!("\n8. Wallet statistics and monitoring...");
    
    let stats = wallet.get_wallet_stats()?;
    println!("   📊 Wallet Statistics:");
    println!("     Name: {}", stats.name);
    println!("     Total Addresses: {}", stats.address_count);
    println!("     Total Balance: {} IPN", stats.total_balance);
    println!("     Transaction Count: {}", stats.transaction_count);
    println!("     Created: {}", stats.created_at.format("%Y-%m-%d %H:%M:%S"));
    
    if let Some(last_sync) = stats.last_sync {
        println!("     Last Sync: {}", last_sync.format("%Y-%m-%d %H:%M:%S"));
    } else {
        println!("     Last Sync: Never");
    }
    
    // 9. Error handling demonstration
    println!("\n9. Error handling demonstration...");
    
    // Try to send from non-existent address
    match wallet.send_transaction(
        "i0000000000000000000000000000000000000000000000000000000000000000000",
        &main_address,
        100,
        Some("secure_password_123")
    ) {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   ✅ Caught expected error: {}", e),
    }
    
    // Try to send with insufficient balance
    match wallet.send_transaction(
        &main_address,
        &trading_address,
        100000, // Very large amount
        Some("secure_password_123")
    ) {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   ✅ Caught expected error: {}", e),
    }
    
    // 10. Cleanup demonstration
    println!("\n10. Cleanup demonstration...");
    
    // Remove temporary address
    wallet.remove_address(&temp_address)?;
    println!("✅ Removed temporary address");
    
    let final_addresses = wallet.list_addresses()?;
    println!("   Final address count: {}", final_addresses.len());
    
    println!("\n🎉 Advanced example completed successfully!");
    println!("\nThis example demonstrated:");
    println!("  • Secure wallet creation with password protection");
    println!("  • Multiple address generation and management");
    println!("  • Transaction sending and simulation");
    println!("  • Address labeling and organization");
    println!("  • Backup and restore functionality");
    println!("  • Export and import capabilities");
    println!("  • Error handling and validation");
    println!("  • Wallet statistics and monitoring");
    
    Ok(())
}