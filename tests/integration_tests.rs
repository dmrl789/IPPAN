use ippan::{
    crypto::KeyPair,
    transaction::Transaction,
    time::IppanTime,
    mempool::Mempool,
    block::{Block, BlockBuilder},
    state::StateManager,
    wallet::WalletManager,
};
use std::sync::Arc;

#[tokio::test]
async fn test_end_to_end_transaction_flow() {
    // Create components
    let ippan_time = Arc::new(IppanTime::new());
    let mempool = Mempool::new(2);
    let state_manager = StateManager::new(10);
    let wallet_manager = WalletManager::new();
    
    // Create wallets
    wallet_manager.create_wallet("alice".to_string()).await.unwrap();
    wallet_manager.create_wallet("bob".to_string()).await.unwrap();
    
    let alice_wallet = wallet_manager.get_wallet(Some("alice")).await.unwrap();
    let bob_wallet = wallet_manager.get_wallet(Some("bob")).await.unwrap();
    
    // Set initial balances
    let alice_pub_hash = ippan::crypto::hash(&alice_wallet.keypair.public_key);
    let bob_pub_hash = ippan::crypto::hash(&bob_wallet.keypair.public_key);
    
    state_manager.set_balance(&alice_pub_hash, 10000).await;
    state_manager.set_balance(&bob_pub_hash, 0).await;
    
    // Create transaction
    let transaction = Transaction::new(
        &alice_wallet.keypair,
        bob_wallet.keypair.public_key,
        5000,
        1,
        ippan_time.clone(),
    ).unwrap();
    
    // Add to mempool
    let added = mempool.add_transaction(transaction.clone()).await.unwrap();
    assert!(added);
    
    // Create block
    let builder = BlockBuilder::new();
    let block = builder.build_block(
        vec![],
        1,
        ippan_time.ippan_time_us().await,
        ippan::crypto::hash(&alice_wallet.keypair.public_key),
        vec![transaction],
    ).unwrap();
    
    // Apply block to state
    let applied = state_manager.apply_block(&block, &[transaction]).await.unwrap();
    assert_eq!(applied, 1);
    
    // Verify balances
    let alice_balance = state_manager.get_balance(&alice_pub_hash).await;
    let bob_balance = state_manager.get_balance(&bob_pub_hash).await;
    
    assert_eq!(alice_balance, 5000);
    assert_eq!(bob_balance, 5000);
}

#[tokio::test]
async fn test_mempool_sharding() {
    let mempool = Mempool::new(4);
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    // Add transactions from different accounts
    for i in 0..100 {
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            100,
            i + 1,
            ippan_time.clone(),
        ).unwrap();
        
        mempool.add_transaction(tx).await.unwrap();
    }
    
    // Verify all transactions are in mempool
    assert_eq!(mempool.get_total_size().await, 100);
    
    // Get transactions for block
    let transactions = mempool.get_transactions_for_block(50).await;
    assert_eq!(transactions.len(), 50);
    
    // Verify remaining transactions
    assert_eq!(mempool.get_total_size().await, 50);
}

#[tokio::test]
async fn test_block_validation() {
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    // Create valid transaction
    let transaction = Transaction::new(
        &keypair,
        recipient.public_key,
        1000,
        1,
        ippan_time.clone(),
    ).unwrap();
    
    // Create block
    let builder = BlockBuilder::new();
    let block = builder.build_block(
        vec![],
        1,
        ippan_time.ippan_time_us().await,
        ippan::crypto::hash(&keypair.public_key),
        vec![transaction],
    ).unwrap();
    
    // Verify block
    assert!(block.verify().unwrap());
    
    // Check block properties
    assert_eq!(block.transaction_count(), 1);
    assert!(!block.is_empty());
}

#[tokio::test]
async fn test_wallet_operations() {
    let wallet_manager = WalletManager::new();
    
    // Create wallet
    wallet_manager.create_wallet("test_wallet".to_string()).await.unwrap();
    
    // Get wallet
    let wallet = wallet_manager.get_wallet(Some("test_wallet")).await.unwrap();
    
    // Check default values
    assert_eq!(wallet.get_balance(), 0);
    assert_eq!(wallet.get_nonce(), 0);
    
    // Update balance
    wallet_manager.update_wallet_balance("test_wallet", 1000).await.unwrap();
    
    // Verify update
    let updated_wallet = wallet_manager.get_wallet(Some("test_wallet")).await.unwrap();
    assert_eq!(updated_wallet.get_balance(), 1000);
    
    // Increment nonce
    wallet_manager.increment_wallet_nonce("test_wallet").await.unwrap();
    
    // Verify nonce increment
    let final_wallet = wallet_manager.get_wallet(Some("test_wallet")).await.unwrap();
    assert_eq!(final_wallet.get_nonce(), 1);
}

#[tokio::test]
async fn test_state_snapshots() {
    let state_manager = StateManager::new(5);
    
    // Add some accounts
    let account1 = [1u8; 32];
    let account2 = [2u8; 32];
    
    state_manager.set_balance(&account1, 1000).await;
    state_manager.set_balance(&account2, 2000).await;
    
    // Create snapshot
    let snapshot = state_manager.create_snapshot(1, 1234567890).await.unwrap();
    
    // Verify snapshot
    assert_eq!(snapshot.round_id, 1);
    assert_eq!(snapshot.accounts.len(), 2);
    assert_eq!(snapshot.accounts[&account1].balance, 1000);
    assert_eq!(snapshot.accounts[&account2].balance, 2000);
    
    // Check state root
    let state_root = state_manager.get_state_root().await;
    assert_ne!(state_root, [0u8; 32]);
}

#[tokio::test]
async fn test_transaction_ordering() {
    let mempool = Mempool::new(1);
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    // Create transactions with different nonces
    let tx1 = Transaction::new(
        &keypair,
        recipient.public_key,
        100,
        1,
        ippan_time.clone(),
    ).unwrap();
    
    let tx2 = Transaction::new(
        &keypair,
        recipient.public_key,
        200,
        2,
        ippan_time.clone(),
    ).unwrap();
    
    // Add transactions in reverse order
    mempool.add_transaction(tx2.clone()).await.unwrap();
    mempool.add_transaction(tx1.clone()).await.unwrap();
    
    // Get transactions for block (should be ordered by nonce)
    let transactions = mempool.get_transactions_for_block(10).await;
    
    // Verify ordering (transactions should be ordered by HashTimer, then by nonce)
    assert_eq!(transactions.len(), 2);
    
    // The first transaction should have nonce 1
    assert_eq!(transactions[0].nonce, 1);
    assert_eq!(transactions[1].nonce, 2);
}
