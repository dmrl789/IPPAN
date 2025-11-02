use ippan_wallet::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize wallet storage
    let storage = Arc::new(storage::WalletStorage::new("./example_wallet"));
    let wallet = operations::WalletManager::new(storage, None);

    println!("🚀 IPPAN Multi-Address Wallet Example");
    println!("=====================================");

    // 1. Create a new wallet
    println!("\n1. Creating wallet...");
    wallet.create_wallet("Example Wallet".to_string(), Some("example_password"))?;
    println!("✅ Wallet created successfully!");

    // 2. Generate multiple addresses
    println!("\n2. Generating addresses...");
    let addresses =
        wallet.generate_addresses(5, Some("Example".to_string()), Some("example_password"))?;

    println!("✅ Generated {} addresses:", addresses.len());
    for (i, addr) in addresses.iter().enumerate() {
        println!("   {}. {}", i + 1, addr);
    }

    // 3. List all addresses with details
    println!("\n3. Wallet addresses:");
    let wallet_addresses = wallet.list_addresses()?;
    for addr in wallet_addresses {
        println!("   Address: {}", addr.address);
        println!("   Label: {}", addr.label.as_deref().unwrap_or("No label"));
        println!(
            "   Created: {}",
            addr.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!("   Balance: {}", addr.balance);
        println!("   Nonce: {}", addr.nonce);
        println!();
    }

    // 4. Get wallet statistics
    println!("4. Wallet statistics:");
    let stats = wallet.get_wallet_stats()?;
    println!("   Name: {}", stats.name);
    println!("   Addresses: {}", stats.address_count);
    println!("   Total Balance: {}", stats.total_balance);
    println!("   Transactions: {}", stats.transaction_count);
    println!(
        "   Created: {}",
        stats.created_at.format("%Y-%m-%d %H:%M:%S")
    );

    // 5. Create a backup
    println!("\n5. Creating backup...");
    let backup_path = wallet.create_backup()?;
    println!("✅ Backup created: {}", backup_path.display());

    // 6. Export wallet data
    println!("\n6. Exporting wallet data...");
    let backup_data = wallet.export_wallet()?;
    println!(
        "✅ Wallet exported ({} addresses)",
        backup_data.wallet_state.addresses.len()
    );

    // 7. Test address management
    println!("\n7. Testing address management...");
    let first_address = &addresses[0];

    // Update address label
    wallet.update_address_label(first_address, Some("Updated Label".to_string()))?;
    println!("✅ Updated label for address: {}", first_address);

    // Get specific address info
    let addr_info = wallet.get_address(first_address)?;
    println!(
        "   New label: {}",
        addr_info.label.as_deref().unwrap_or("No label")
    );

    // 8. Test transaction simulation (without RPC)
    println!("\n8. Testing transaction simulation...");
    if addresses.len() >= 2 {
        let from_addr = &addresses[0];
        let to_addr = &addresses[1];

        println!(
            "   Simulating transaction from {} to {}",
            from_addr, to_addr
        );
        println!("   Note: This is a simulation without RPC connection");

        // In a real scenario, this would send an actual transaction
        // let tx_hash = wallet.send_transaction(from_addr, to_addr, 100, Some("example_password"))?;
        // println!("   Transaction sent: {}", tx_hash);
    }

    println!("\n🎉 Example completed successfully!");
    println!("\nTo use the CLI tool, run:");
    println!("   cargo run --bin ippan-wallet -- --help");

    Ok(())
}
