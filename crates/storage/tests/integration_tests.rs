//! Integration tests for storage backends (Sled and in-memory).
//! Tests block storage, account management, transactions, L2 operations,
//! round certificates, validator telemetry, and chain state.

use ippan_storage::{Account, ChainState, MemoryStorage, SledStorage, Storage, ValidatorTelemetry};
use ippan_types::{
    Amount, Block, IppanTimeMicros, L2Commit, L2ExitRecord, L2ExitStatus, L2Network,
    L2NetworkStatus, RoundCertificate, RoundFinalizationRecord, RoundWindow, Transaction,
};
use tempfile::TempDir;

/// Helper to create a test block
fn create_test_block(round: u64, creator: [u8; 32], parent_ids: Vec<[u8; 32]>) -> Block {
    Block::new(parent_ids, vec![], round, creator)
}

/// Helper to create a test transaction
fn create_test_transaction(
    from: [u8; 32],
    to: [u8; 32],
    amount_val: u64,
    nonce: u64,
) -> Transaction {
    Transaction::new(from, to, Amount::from_micro_ipn(amount_val), nonce)
}

/// Helper to create a test account
fn create_test_account(address: [u8; 32], balance: u64, nonce: u64) -> Account {
    Account {
        address,
        balance,
        nonce,
    }
}

/// Helper to create test L2 network
fn create_test_l2_network(id: &str) -> L2Network {
    L2Network {
        id: id.to_string(),
        proof_type: "zk".to_string(),
        da_mode: "inline".to_string(),
        status: L2NetworkStatus::Active,
        last_epoch: 0,
        total_commits: 0,
        total_exits: 0,
        last_commit_time: None,
        registered_at: 1000,
        challenge_window_ms: Some(3600000),
    }
}

/// Helper to create test L2 commit
fn create_test_l2_commit(id: &str, l2_id: &str, state_root: &str) -> L2Commit {
    L2Commit {
        id: id.to_string(),
        l2_id: l2_id.to_string(),
        epoch: 1,
        state_root: state_root.to_string(),
        da_hash: "0xabcd".to_string(),
        proof_type: "zk".to_string(),
        proof: None,
        inline_data: None,
        submitted_at: 5000,
        hashtimer: "test_timer".to_string(),
    }
}

/// Helper to create test L2 exit record
fn create_test_l2_exit(id: &str, l2_id: &str, account: &str) -> L2ExitRecord {
    L2ExitRecord {
        id: id.to_string(),
        l2_id: l2_id.to_string(),
        epoch: 1,
        account: account.to_string(),
        amount: 1000.0,
        nonce: Some(1),
        proof_of_inclusion: "proof123".to_string(),
        status: L2ExitStatus::Pending,
        submitted_at: 6000,
        finalized_at: None,
        rejection_reason: None,
        challenge_window_ends_at: None,
    }
}

/// Helper to create test round certificate
fn create_test_round_cert(round: u64) -> RoundCertificate {
    RoundCertificate {
        round,
        block_ids: vec![[1u8; 32], [2u8; 32]],
        agg_sig: vec![10, 11, 12],
    }
}

/// Helper to create test round finalization
fn create_test_round_finalization(round: u64) -> RoundFinalizationRecord {
    RoundFinalizationRecord {
        round,
        window: RoundWindow {
            id: round,
            start_us: IppanTimeMicros(7000),
            end_us: IppanTimeMicros(8000),
        },
        ordered_tx_ids: vec![],
        fork_drops: vec![],
        state_root: [3u8; 32],
        proof: create_test_round_cert(round),
    }
}

/// Helper to create test validator telemetry
fn create_test_validator_telemetry(validator_id: [u8; 32]) -> ValidatorTelemetry {
    ValidatorTelemetry {
        validator_id,
        blocks_proposed: 10,
        blocks_verified: 20,
        rounds_active: 5,
        avg_latency_us: 1000,
        slash_count: 0,
        stake: 10000,
        age_rounds: 100,
        last_active_round: 5,
        uptime_percentage: 99.5,
        recent_performance: 0.98,
        network_contribution: 0.95,
    }
}

// ============================================================================
// Generic test suite that works with any Storage implementation
// ============================================================================

fn test_block_storage<S: Storage>(storage: &S) {
    let creator1 = [1u8; 32];

    let block1 = create_test_block(1, creator1, vec![]);
    let block1_hash = block1.hash();

    // Store and retrieve block
    storage.store_block(block1.clone()).unwrap();
    let retrieved = storage.get_block(&block1_hash).unwrap();
    assert!(retrieved.is_some());
    let retrieved_block = retrieved.unwrap();
    assert_eq!(retrieved_block.header.round, 1);

    // Get block by height
    let by_height = storage.get_block_by_height(1).unwrap();
    assert!(by_height.is_some());
    assert_eq!(by_height.unwrap().header.round, 1);

    // Latest height should be updated
    let latest_height = storage.get_latest_height().unwrap();
    assert_eq!(latest_height, 1);

    // Store another block
    let block2 = create_test_block(2, creator1, vec![block1_hash]);
    storage.store_block(block2).unwrap();
    let latest_height = storage.get_latest_height().unwrap();
    assert_eq!(latest_height, 2);
}

fn test_transaction_storage<S: Storage>(storage: &S) {
    let addr1 = [10u8; 32];
    let addr2 = [20u8; 32];
    let addr3 = [30u8; 32];

    let tx1 = create_test_transaction(addr1, addr2, 100, 1);
    let tx2 = create_test_transaction(addr2, addr3, 200, 2);
    let tx3 = create_test_transaction(addr1, addr3, 300, 3);

    let tx1_hash = tx1.hash();

    // Store transactions
    storage.store_transaction(tx1.clone()).unwrap();
    storage.store_transaction(tx2.clone()).unwrap();
    storage.store_transaction(tx3.clone()).unwrap();

    // Retrieve by hash
    let retrieved = storage.get_transaction(&tx1_hash).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().amount, Amount::from_micro_ipn(100));

    // Get transaction count
    let count = storage.get_transaction_count().unwrap();
    assert_eq!(count, 3);

    // Get transactions by address
    let addr1_txs = storage.get_transactions_by_address(&addr1).unwrap();
    assert_eq!(addr1_txs.len(), 2); // tx1 (from), tx3 (from)

    let addr2_txs = storage.get_transactions_by_address(&addr2).unwrap();
    assert_eq!(addr2_txs.len(), 2); // tx1 (to), tx2 (from)

    let addr3_txs = storage.get_transactions_by_address(&addr3).unwrap();
    assert_eq!(addr3_txs.len(), 2); // tx2 (to), tx3 (to)
}

fn test_account_storage<S: Storage>(storage: &S) {
    let addr1 = [100u8; 32];
    let addr2 = [200u8; 32];

    let acc1 = create_test_account(addr1, 1000, 5);
    let acc2 = create_test_account(addr2, 2000, 10);

    // Update accounts
    storage.update_account(acc1.clone()).unwrap();
    storage.update_account(acc2.clone()).unwrap();

    // Retrieve accounts
    let retrieved1 = storage.get_account(&addr1).unwrap();
    assert!(retrieved1.is_some());
    let acc1_data = retrieved1.unwrap();
    assert_eq!(acc1_data.balance, 1000);
    assert_eq!(acc1_data.nonce, 5);

    let retrieved2 = storage.get_account(&addr2).unwrap();
    assert!(retrieved2.is_some());
    assert_eq!(retrieved2.unwrap().balance, 2000);

    // Get all accounts
    let all_accounts = storage.get_all_accounts().unwrap();
    assert!(all_accounts.len() >= 2);

    // Update existing account
    let acc1_updated = create_test_account(addr1, 1500, 6);
    storage.update_account(acc1_updated).unwrap();
    let retrieved_updated = storage.get_account(&addr1).unwrap().unwrap();
    assert_eq!(retrieved_updated.balance, 1500);
    assert_eq!(retrieved_updated.nonce, 6);

    // Non-existent account
    let non_existent = storage.get_account(&[255u8; 32]).unwrap();
    assert!(non_existent.is_none());
}

fn test_l2_network_storage<S: Storage>(storage: &S) {
    let net1 = create_test_l2_network("net1");
    let net2 = create_test_l2_network("net2");

    // Store networks
    storage.put_l2_network(net1.clone()).unwrap();
    storage.put_l2_network(net2.clone()).unwrap();

    // Retrieve by id
    let retrieved = storage.get_l2_network("net1").unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "net1");

    // List all networks
    let all_networks = storage.list_l2_networks().unwrap();
    assert_eq!(all_networks.len(), 2);

    // Non-existent network
    let non_existent = storage.get_l2_network("non-existent").unwrap();
    assert!(non_existent.is_none());
}

fn test_l2_commit_storage<S: Storage>(storage: &S) {
    let commit1 = create_test_l2_commit("commit1", "net1", "root1");
    let commit2 = create_test_l2_commit("commit2", "net1", "root2");
    let commit3 = create_test_l2_commit("commit3", "net2", "root3");

    // Store commits
    storage.store_l2_commit(commit1.clone()).unwrap();
    storage.store_l2_commit(commit2.clone()).unwrap();
    storage.store_l2_commit(commit3.clone()).unwrap();

    // List all commits
    let all_commits = storage.list_l2_commits(None).unwrap();
    assert_eq!(all_commits.len(), 3);

    // List commits for specific network
    let net1_commits = storage.list_l2_commits(Some("net1")).unwrap();
    assert_eq!(net1_commits.len(), 2);

    let net2_commits = storage.list_l2_commits(Some("net2")).unwrap();
    assert_eq!(net2_commits.len(), 1);
}

fn test_l2_exit_storage<S: Storage>(storage: &S) {
    let exit1 = create_test_l2_exit("exit1", "net1", "account1");
    let exit2 = create_test_l2_exit("exit2", "net1", "account2");
    let exit3 = create_test_l2_exit("exit3", "net2", "account3");

    // Store exits
    storage.store_l2_exit(exit1.clone()).unwrap();
    storage.store_l2_exit(exit2.clone()).unwrap();
    storage.store_l2_exit(exit3.clone()).unwrap();

    // List all exits
    let all_exits = storage.list_l2_exits(None).unwrap();
    assert_eq!(all_exits.len(), 3);

    // List exits for specific network
    let net1_exits = storage.list_l2_exits(Some("net1")).unwrap();
    assert_eq!(net1_exits.len(), 2);

    let net2_exits = storage.list_l2_exits(Some("net2")).unwrap();
    assert_eq!(net2_exits.len(), 1);
}

fn test_round_certificate_storage<S: Storage>(storage: &S) {
    let cert1 = create_test_round_cert(10);
    let cert2 = create_test_round_cert(20);

    // Store certificates
    storage.store_round_certificate(cert1.clone()).unwrap();
    storage.store_round_certificate(cert2.clone()).unwrap();

    // Retrieve certificates
    let retrieved1 = storage.get_round_certificate(10).unwrap();
    assert!(retrieved1.is_some());
    assert_eq!(retrieved1.unwrap().round, 10);

    let retrieved2 = storage.get_round_certificate(20).unwrap();
    assert!(retrieved2.is_some());
    assert_eq!(retrieved2.unwrap().block_ids.len(), 2);

    // Non-existent certificate
    let non_existent = storage.get_round_certificate(999).unwrap();
    assert!(non_existent.is_none());
}

fn test_round_finalization_storage<S: Storage>(storage: &S) {
    let fin1 = create_test_round_finalization(15);
    let fin2 = create_test_round_finalization(25);
    let fin3 = create_test_round_finalization(35);

    // Store finalizations in order
    storage.store_round_finalization(fin1.clone()).unwrap();
    storage.store_round_finalization(fin2.clone()).unwrap();
    storage.store_round_finalization(fin3.clone()).unwrap();

    // Retrieve by round
    let retrieved1 = storage.get_round_finalization(15).unwrap();
    assert!(retrieved1.is_some());
    assert_eq!(retrieved1.unwrap().round, 15);

    // Get latest finalization
    let latest = storage.get_latest_round_finalization().unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().round, 35);

    // Non-existent finalization
    let non_existent = storage.get_round_finalization(999).unwrap();
    assert!(non_existent.is_none());
}

fn test_chain_state_storage<S: Storage>(storage: &S) {
    // Get initial state (should be default)
    let initial_state = storage.get_chain_state().unwrap();
    assert_eq!(initial_state.total_issued_micro, 0);
    assert_eq!(initial_state.last_updated_round, 0);

    // Update chain state
    let mut state = ChainState {
        total_issued_micro: 1000000,
        last_updated_round: 50,
    };
    storage.update_chain_state(&state).unwrap();

    // Retrieve updated state
    let retrieved = storage.get_chain_state().unwrap();
    assert_eq!(retrieved.total_issued_micro, 1000000);
    assert_eq!(retrieved.last_updated_round, 50);

    // Update again with helper methods
    state.add_issued_micro(500000);
    state.update_round(51);
    storage.update_chain_state(&state).unwrap();

    let retrieved2 = storage.get_chain_state().unwrap();
    assert_eq!(retrieved2.total_issued_micro, 1500000);
    assert_eq!(retrieved2.last_updated_round, 51);
}

fn test_validator_telemetry_storage<S: Storage>(storage: &S) {
    let val1_id = [101u8; 32];
    let val2_id = [102u8; 32];

    let tel1 = create_test_validator_telemetry(val1_id);
    let tel2 = create_test_validator_telemetry(val2_id);

    // Store telemetry
    storage.store_validator_telemetry(&val1_id, &tel1).unwrap();
    storage.store_validator_telemetry(&val2_id, &tel2).unwrap();

    // Retrieve individual telemetry
    let retrieved1 = storage.get_validator_telemetry(&val1_id).unwrap();
    assert!(retrieved1.is_some());
    assert_eq!(retrieved1.unwrap().blocks_proposed, 10);

    // Get all validator telemetry
    let all_telemetry = storage.get_all_validator_telemetry().unwrap();
    assert_eq!(all_telemetry.len(), 2);
    assert!(all_telemetry.contains_key(&val1_id));
    assert!(all_telemetry.contains_key(&val2_id));

    // Update existing telemetry
    let mut tel1_updated = tel1.clone();
    tel1_updated.blocks_proposed = 15;
    tel1_updated.blocks_verified = 25;
    storage
        .store_validator_telemetry(&val1_id, &tel1_updated)
        .unwrap();

    let retrieved_updated = storage.get_validator_telemetry(&val1_id).unwrap().unwrap();
    assert_eq!(retrieved_updated.blocks_proposed, 15);
    assert_eq!(retrieved_updated.blocks_verified, 25);

    // Non-existent validator
    let non_existent = storage.get_validator_telemetry(&[255u8; 32]).unwrap();
    assert!(non_existent.is_none());
}

// ============================================================================
// Memory storage tests
// ============================================================================

#[test]
fn memory_storage_blocks() {
    let storage = MemoryStorage::new();
    test_block_storage(&storage);
}

#[test]
fn memory_storage_transactions() {
    let storage = MemoryStorage::new();
    test_transaction_storage(&storage);
}

#[test]
fn memory_storage_accounts() {
    let storage = MemoryStorage::new();
    test_account_storage(&storage);
}

#[test]
fn memory_storage_l2_networks() {
    let storage = MemoryStorage::new();
    test_l2_network_storage(&storage);
}

#[test]
fn memory_storage_l2_commits() {
    let storage = MemoryStorage::new();
    test_l2_commit_storage(&storage);
}

#[test]
fn memory_storage_l2_exits() {
    let storage = MemoryStorage::new();
    test_l2_exit_storage(&storage);
}

#[test]
fn memory_storage_round_certificates() {
    let storage = MemoryStorage::new();
    test_round_certificate_storage(&storage);
}

#[test]
fn memory_storage_round_finalizations() {
    let storage = MemoryStorage::new();
    test_round_finalization_storage(&storage);
}

#[test]
fn memory_storage_chain_state() {
    let storage = MemoryStorage::new();
    test_chain_state_storage(&storage);
}

#[test]
fn memory_storage_validator_telemetry() {
    let storage = MemoryStorage::new();
    test_validator_telemetry_storage(&storage);
}

// ============================================================================
// Sled storage tests
// ============================================================================

#[test]
fn sled_storage_blocks() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    storage.initialize().unwrap();
    test_block_storage(&storage);
}

#[test]
fn sled_storage_transactions() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_transaction_storage(&storage);
}

#[test]
fn sled_storage_accounts() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_account_storage(&storage);
}

#[test]
fn sled_storage_l2_networks() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_l2_network_storage(&storage);
}

#[test]
fn sled_storage_l2_commits() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_l2_commit_storage(&storage);
}

#[test]
fn sled_storage_l2_exits() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_l2_exit_storage(&storage);
}

#[test]
fn sled_storage_round_certificates() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_round_certificate_storage(&storage);
}

#[test]
fn sled_storage_round_finalizations() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_round_finalization_storage(&storage);
}

#[test]
fn sled_storage_chain_state() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_chain_state_storage(&storage);
}

#[test]
fn sled_storage_validator_telemetry() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();
    test_validator_telemetry_storage(&storage);
}

// ============================================================================
// Sled-specific persistence tests
// ============================================================================

#[test]
fn sled_storage_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    // Create storage and store data
    {
        let storage = SledStorage::new(&path).unwrap();
        let addr = [77u8; 32];
        let acc = create_test_account(addr, 5000, 10);
        storage.update_account(acc).unwrap();

        let block = create_test_block(1, [0u8; 32], vec![]);
        storage.store_block(block).unwrap();

        storage.flush().unwrap();
    }

    // Reopen storage and verify data persisted
    {
        let storage = SledStorage::new(&path).unwrap();
        let addr = [77u8; 32];
        let retrieved_acc = storage.get_account(&addr).unwrap();
        assert!(retrieved_acc.is_some());
        assert_eq!(retrieved_acc.unwrap().balance, 5000);

        let height = storage.get_latest_height().unwrap();
        assert_eq!(height, 1);
    }
}

#[test]
fn sled_storage_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let storage = SledStorage::new(temp_dir.path()).unwrap();

    // Before initialization, height should be 0
    let height_before = storage.get_latest_height().unwrap();
    assert_eq!(height_before, 0);

    // Initialize creates genesis block and account
    storage.initialize().unwrap();

    let height_after = storage.get_latest_height().unwrap();
    assert_eq!(height_after, 0); // Genesis is at height 0

    let genesis_block = storage.get_block_by_height(0).unwrap();
    assert!(genesis_block.is_some());

    let genesis_account = storage.get_account(&[0u8; 32]).unwrap();
    assert!(genesis_account.is_some());
    assert_eq!(genesis_account.unwrap().balance, 1_000_000);
}

// ============================================================================
// Edge case and stress tests
// ============================================================================

#[test]
fn test_multiple_blocks_same_storage() {
    let storage = MemoryStorage::new();
    let creator = [1u8; 32];

    // Store multiple blocks
    for i in 0..10 {
        let parents = if i > 0 {
            vec![[i as u8 - 1; 32]]
        } else {
            vec![]
        };
        let block = create_test_block(i, creator, parents);
        storage.store_block(block).unwrap();
    }

    let latest = storage.get_latest_height().unwrap();
    assert_eq!(latest, 9);

    // Verify all blocks can be retrieved
    for i in 0..10 {
        let block = storage.get_block_by_height(i).unwrap();
        assert!(block.is_some());
    }
}

#[test]
fn test_large_transaction_set() {
    let storage = MemoryStorage::new();
    let addr1 = [1u8; 32];

    // Store many transactions
    for i in 0..100 {
        let to_addr = [i as u8; 32];
        let tx = create_test_transaction(addr1, to_addr, 100 + i as u64, i as u64);
        storage.store_transaction(tx).unwrap();
    }

    let count = storage.get_transaction_count().unwrap();
    assert_eq!(count, 100);

    let addr1_txs = storage.get_transactions_by_address(&addr1).unwrap();
    assert_eq!(addr1_txs.len(), 100); // All from addr1
}

#[test]
fn test_concurrent_storage_access() {
    use std::sync::Arc;
    use std::thread;

    let storage = Arc::new(MemoryStorage::new());
    let mut handles = vec![];

    // Spawn multiple threads writing to storage
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = thread::spawn(move || {
            let addr = [i as u8; 32];
            let acc = create_test_account(addr, 1000 * i as u64, i as u64);
            storage_clone.update_account(acc).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all accounts were stored
    let all_accounts = storage.get_all_accounts().unwrap();
    assert_eq!(all_accounts.len(), 10);
}
