use ippan_wallet::*;
use std::sync::Arc;
use tempfile::tempdir;

struct MockRpcClient;

impl operations::RpcClient for MockRpcClient {
    fn get_balance(&self, _address: &str) -> Result<u64> {
        Ok(1000)
    }

    fn get_nonce(&self, _address: &str) -> Result<u64> {
        Ok(0)
    }

    fn send_transaction(&self, _transaction: &ippan_types::Transaction) -> Result<String> {
        Ok("mock_tx_hash_12345".to_string())
    }

    fn get_transaction(&self, _tx_hash: &str) -> Result<Option<ippan_types::Transaction>> {
        Ok(None)
    }

    fn get_transactions_by_address(&self, _address: &str) -> Result<Vec<ippan_types::Transaction>> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_wallet_lifecycle() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let rpc_client = Arc::new(MockRpcClient);
    let wallet = operations::WalletManager::new(storage, Some(rpc_client));

    // Test wallet creation
    wallet
        .create_wallet("Test Wallet".to_string(), Some("password123"))
        .unwrap();

    // Test address generation
    let address1 = wallet
        .generate_address(Some("Test Address 1".to_string()), Some("password123"))
        .unwrap();
    let address2 = wallet
        .generate_address(Some("Test Address 2".to_string()), Some("password123"))
        .unwrap();

    assert!(address1.starts_with('i'));
    assert!(address2.starts_with('i'));
    assert_eq!(address1.len(), 65);
    assert_eq!(address2.len(), 65);
    assert_ne!(address1, address2);

    // Test listing addresses
    let addresses = wallet.list_addresses().unwrap();
    assert_eq!(addresses.len(), 2);

    // Test balance checking
    let balance1 = wallet.get_address_balance(&address1).unwrap();
    let balance2 = wallet.get_address_balance(&address2).unwrap();
    assert_eq!(balance1, 1000); // Mock RPC returns 1000
    assert_eq!(balance2, 1000);

    // Test total balance
    let total_balance = wallet.get_total_balance().unwrap();
    assert_eq!(total_balance, 2000);

    // Test transaction sending
    let tx_hash = wallet
        .send_transaction(&address1, &address2, 100, Some("password123"))
        .unwrap();
    assert_eq!(tx_hash, "mock_tx_hash_12345");

    // Test wallet stats
    let stats = wallet.get_wallet_stats().unwrap();
    assert_eq!(stats.name, "Test Wallet");
    assert_eq!(stats.address_count, 2);
    assert_eq!(stats.total_balance, 2000);
}

#[tokio::test]
async fn test_multiple_address_generation() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let wallet = operations::WalletManager::new(storage, None);

    wallet
        .create_wallet("Test Wallet".to_string(), Some("password123"))
        .unwrap();

    // Generate 10 addresses
    let addresses = wallet
        .generate_addresses(10, Some("Batch".to_string()), Some("password123"))
        .unwrap();
    assert_eq!(addresses.len(), 10);

    // Verify all addresses are unique
    let mut unique_addresses = std::collections::HashSet::new();
    for addr in &addresses {
        assert!(unique_addresses.insert(addr));
        assert!(addr.starts_with('i'));
        assert_eq!(addr.len(), 65);
    }

    // Check wallet state
    let wallet_addresses = wallet.list_addresses().unwrap();
    assert_eq!(wallet_addresses.len(), 10);

    for (i, wallet_addr) in wallet_addresses.iter().enumerate() {
        assert_eq!(wallet_addr.label, Some(format!("Batch_{}", i + 1)));
    }
}

#[tokio::test]
async fn test_wallet_backup_restore() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let wallet = operations::WalletManager::new(storage, None);

    // Create wallet with some data
    wallet
        .create_wallet("Test Wallet".to_string(), Some("password123"))
        .unwrap();
    let address1 = wallet
        .generate_address(Some("Address 1".to_string()), Some("password123"))
        .unwrap();
    let address2 = wallet
        .generate_address(Some("Address 2".to_string()), Some("password123"))
        .unwrap();

    // Create backup
    let backup_path = wallet.create_backup().unwrap();
    assert!(backup_path.exists());

    // Create new wallet instance and restore
    let temp_dir2 = tempdir().unwrap();
    let storage2 = Arc::new(storage::WalletStorage::new(temp_dir2.path()));
    let wallet2 = operations::WalletManager::new(storage2, None);

    wallet2
        .restore_from_backup(&backup_path, Some("password123"))
        .unwrap();

    // Verify restored data
    let addresses = wallet2.list_addresses().unwrap();
    assert_eq!(addresses.len(), 2);

    let address_labels: Vec<Option<&String>> = addresses.iter().map(|a| a.label.as_ref()).collect();
    assert!(address_labels.contains(&Some(&"Address 1".to_string())));
    assert!(address_labels.contains(&Some(&"Address 2".to_string())));
}

#[tokio::test]
async fn test_wallet_encryption() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let wallet = operations::WalletManager::new(storage, None);

    // Create wallet with password
    wallet
        .create_wallet("Encrypted Wallet".to_string(), Some("secure_password"))
        .unwrap();

    // Generate address with password
    let address = wallet
        .generate_address(
            Some("Encrypted Address".to_string()),
            Some("secure_password"),
        )
        .unwrap();

    // Verify address was created
    let addresses = wallet.list_addresses().unwrap();
    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses[0].address, address);

    // Test that wrong password fails
    let result = wallet.generate_address(Some("Test".to_string()), Some("wrong_password"));
    assert!(result.is_err());
}

#[tokio::test]
async fn test_address_management() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let wallet = operations::WalletManager::new(storage, None);

    wallet
        .create_wallet("Test Wallet".to_string(), Some("password123"))
        .unwrap();

    // Generate addresses
    let address1 = wallet
        .generate_address(Some("Address 1".to_string()), Some("password123"))
        .unwrap();
    let address2 = wallet
        .generate_address(Some("Address 2".to_string()), Some("password123"))
        .unwrap();

    // Test address retrieval
    let addr1 = wallet.get_address(&address1).unwrap();
    assert_eq!(addr1.label, Some("Address 1".to_string()));

    // Test label update
    wallet
        .update_address_label(&address1, Some("Updated Label".to_string()))
        .unwrap();
    let updated_addr1 = wallet.get_address(&address1).unwrap();
    assert_eq!(updated_addr1.label, Some("Updated Label".to_string()));

    // Test address removal
    wallet.remove_address(&address2).unwrap();
    let addresses = wallet.list_addresses().unwrap();
    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses[0].address, address1);
}

#[tokio::test]
async fn test_transaction_history() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let rpc_client = Arc::new(MockRpcClient);
    let wallet = operations::WalletManager::new(storage, Some(rpc_client));

    wallet
        .create_wallet("Test Wallet".to_string(), Some("password123"))
        .unwrap();

    let address1 = wallet
        .generate_address(Some("Sender".to_string()), Some("password123"))
        .unwrap();
    let address2 = wallet
        .generate_address(Some("Receiver".to_string()), Some("password123"))
        .unwrap();

    // Send transaction
    let tx_hash = wallet
        .send_transaction(&address1, &address2, 100, Some("password123"))
        .unwrap();

    // Check transaction history
    let history1 = wallet.get_address_transactions(&address1).unwrap();
    let history2 = wallet.get_address_transactions(&address2).unwrap();

    // Note: Mock RPC returns empty history, but we should have cached transaction
    let all_history = wallet.get_all_transactions().unwrap();

    // The transaction should be in the cache
    assert!(!all_history.is_empty());
}

#[tokio::test]
async fn test_wallet_export_import() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let wallet = operations::WalletManager::new(storage, None);

    // Create wallet with data
    wallet
        .create_wallet("Export Test Wallet".to_string(), Some("password123"))
        .unwrap();
    let address = wallet
        .generate_address(Some("Test Address".to_string()), Some("password123"))
        .unwrap();

    // Export wallet
    let backup = wallet.export_wallet().unwrap();
    assert_eq!(backup.wallet_state.addresses.len(), 1);
    assert!(backup.verify_checksum());

    // Import into new wallet
    let temp_dir2 = tempdir().unwrap();
    let storage2 = Arc::new(storage::WalletStorage::new(temp_dir2.path()));
    let wallet2 = operations::WalletManager::new(storage2, None);

    wallet2.import_wallet(backup, Some("password123")).unwrap();

    // Verify imported data
    let addresses = wallet2.list_addresses().unwrap();
    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses[0].address, address);
    assert_eq!(addresses[0].label, Some("Test Address".to_string()));
}

#[tokio::test]
async fn test_error_handling() {
    let temp_dir = tempdir().unwrap();
    let storage = Arc::new(storage::WalletStorage::new(temp_dir.path()));
    let wallet = operations::WalletManager::new(storage, None);

    // Test operations on uninitialized wallet
    let result = wallet.list_addresses();
    assert!(result.is_err());

    // Test operations on locked wallet
    wallet
        .create_wallet("Test Wallet".to_string(), Some("password123"))
        .unwrap();

    // Test sending from non-existent address
    let result = wallet.send_transaction(
        "invalid_address",
        "another_invalid",
        100,
        Some("password123"),
    );
    assert!(result.is_err());

    // Test sending with insufficient balance
    let address = wallet.generate_address(None, Some("password123")).unwrap();
    let result = wallet.send_transaction(
        &address,
        "i1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        2000,
        Some("password123"),
    );
    assert!(result.is_err());
}
