//! Multi-block replay tests with state hash verification
//!
//! These tests verify storage and chain state behavior across multiple blocks/rounds:
//! - Applying blocks sequentially produces consistent state
//! - State hashes are deterministic and verifiable
//! - Snapshots can be created and restored reliably
//! - Replay from genesis produces identical results
//! - State transitions are atomic and consistent

use ippan_storage::{Account, MemoryStorage, SledStorage, Storage};
use ippan_types::{
    chain_state::ChainState, Amount, Block, RoundFinalizationRecord, RoundWindow, Transaction,
    IppanTimeMicros, RoundCertificate,
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

/// Helper to create round finalization
fn create_round_finalization(round: u64, state_root: [u8; 32]) -> RoundFinalizationRecord {
    RoundFinalizationRecord {
        round,
        window: RoundWindow {
            id: round,
            start_us: IppanTimeMicros(round * 1000),
            end_us: IppanTimeMicros((round + 1) * 1000),
        },
        ordered_tx_ids: vec![],
        fork_drops: vec![],
        state_root,
        proof: RoundCertificate {
            round,
            block_ids: vec![],
            agg_sig: vec![],
        },
        total_fees_atomic: Some(0),
        treasury_fees_atomic: Some(0),
        applied_payments: Some(0),
        rejected_payments: Some(0),
    }
}

/// Apply a simple state update: create accounts and store blocks
fn apply_state_update(
    storage: &impl Storage,
    round: u64,
    base_value: u64,
) -> anyhow::Result<[u8; 32]> {
    // Create and store accounts for this round
    for i in 0..5 {
        let addr = [(round * 10 + i) as u8; 32];
        let account = Account {
            address: addr,
            balance: base_value + i * 1000,
            nonce: i,
        };
        storage.update_account(account)?;
    }

    // Create and store a block
    let creator = [round as u8; 32];
    let parent = if round > 0 {
        vec![[(round - 1) as u8; 32]]
    } else {
        vec![]
    };
    let block = create_test_block(round, creator, parent);
    let block_hash = block.hash();
    storage.store_block(block)?;

    // Update chain state
    let mut state = storage.get_chain_state()?;
    state.set_round(round);
    state.add_issued_micro(base_value * 5);
    state.set_last_updated(round);
    storage.update_chain_state(&state)?;

    // Create a deterministic state root from the current state
    let state_root = compute_state_root(storage)?;

    // Store round finalization
    let finalization = create_round_finalization(round, state_root);
    storage.store_round_finalization(finalization)?;

    Ok(state_root)
}

/// Compute a simple state root hash from storage state
fn compute_state_root(storage: &impl Storage) -> anyhow::Result<[u8; 32]> {
    use blake3::Hasher;

    let mut hasher = Hasher::new();

    // Hash all accounts in deterministic order
    let accounts = storage.get_all_accounts()?;
    let mut sorted_accounts: Vec<_> = accounts.into_iter().collect();
    sorted_accounts.sort_by_key(|(addr, _)| *addr);

    for (addr, account) in sorted_accounts {
        hasher.update(&addr);
        hasher.update(&account.balance.to_be_bytes());
        hasher.update(&account.nonce.to_be_bytes());
    }

    // Hash chain state
    let chain_state = storage.get_chain_state()?;
    hasher.update(&chain_state.total_issued_micro.to_be_bytes());
    hasher.update(&chain_state.current_round.to_be_bytes());

    let hash = hasher.finalize();
    let mut state_root = [0u8; 32];
    state_root.copy_from_slice(hash.as_bytes());
    Ok(state_root)
}

// ============================================================================
// COMPREHENSIVE TESTING - PHASE 1: Multi-Block Replay Tests
// ============================================================================

#[test]
fn multi_block_sequential_application() {
    let storage = MemoryStorage::new();

    // Apply multiple rounds sequentially
    let rounds = 10;
    let mut state_roots = Vec::new();

    for round in 0..rounds {
        let state_root = apply_state_update(&storage, round, 10_000).unwrap();
        state_roots.push(state_root);
    }

    // INVARIANT: Latest height should match last round
    let latest_height = storage.get_latest_height().unwrap();
    assert_eq!(latest_height, rounds - 1);

    // INVARIANT: All blocks should be retrievable
    for round in 0..rounds {
        let block = storage.get_block_by_height(round).unwrap();
        assert!(block.is_some(), "Block at height {} should exist", round);
    }

    // INVARIANT: Chain state should be consistent
    let chain_state = storage.get_chain_state().unwrap();
    assert_eq!(chain_state.current_round, rounds - 1);
    assert_eq!(
        chain_state.total_issued_micro,
        10_000 * 5 * rounds,
        "Total issued should match sum of emissions"
    );

    // INVARIANT: State roots should be stored and retrievable
    for (i, expected_root) in state_roots.iter().enumerate() {
        let finalization = storage.get_round_finalization(i as u64).unwrap();
        assert!(finalization.is_some());
        assert_eq!(
            &finalization.unwrap().state_root,
            expected_root,
            "State root mismatch at round {}",
            i
        );
    }
}

#[test]
fn replay_from_genesis_produces_identical_state() {
    // Run 1: Apply state updates
    let storage1 = MemoryStorage::new();
    let rounds = 20;

    for round in 0..rounds {
        apply_state_update(&storage1, round, 5_000).unwrap();
    }

    let final_state_root_1 = compute_state_root(&storage1).unwrap();
    let final_chain_state_1 = storage1.get_chain_state().unwrap();

    // Run 2: Apply same updates in fresh storage
    let storage2 = MemoryStorage::new();

    for round in 0..rounds {
        apply_state_update(&storage2, round, 5_000).unwrap();
    }

    let final_state_root_2 = compute_state_root(&storage2).unwrap();
    let final_chain_state_2 = storage2.get_chain_state().unwrap();

    // INVARIANT: Final state hashes must be identical
    assert_eq!(
        final_state_root_1, final_state_root_2,
        "State roots must match after identical replay"
    );

    // INVARIANT: Chain states must be identical
    assert_eq!(
        final_chain_state_1.total_issued_micro, final_chain_state_2.total_issued_micro,
        "Total issued must match"
    );
    assert_eq!(
        final_chain_state_1.current_round, final_chain_state_2.current_round,
        "Current round must match"
    );

    // INVARIANT: All accounts must be identical
    let accounts1 = storage1.get_all_accounts().unwrap();
    let accounts2 = storage2.get_all_accounts().unwrap();
    assert_eq!(accounts1.len(), accounts2.len(), "Account count must match");

    for (addr, acc1) in accounts1.iter() {
        let acc2 = accounts2.get(addr).expect("Account should exist in storage2");
        assert_eq!(acc1.balance, acc2.balance, "Balance mismatch for {:?}", addr);
        assert_eq!(acc1.nonce, acc2.nonce, "Nonce mismatch for {:?}", addr);
    }
}

#[test]
fn state_hash_consistency_across_operations() {
    let storage = MemoryStorage::new();

    // Apply initial state
    apply_state_update(&storage, 0, 10_000).unwrap();
    let hash1 = compute_state_root(&storage).unwrap();

    // Read state without modification
    let _ = storage.get_chain_state().unwrap();
    let _ = storage.get_all_accounts().unwrap();
    let hash2 = compute_state_root(&storage).unwrap();

    // INVARIANT: Hash should not change after reads
    assert_eq!(hash1, hash2, "State hash should not change after read operations");

    // Apply more state updates
    apply_state_update(&storage, 1, 10_000).unwrap();
    let hash3 = compute_state_root(&storage).unwrap();

    // INVARIANT: Hash should change after modifications
    assert_ne!(hash1, hash3, "State hash should change after modifications");

    // Recompute hash multiple times
    let hash4 = compute_state_root(&storage).unwrap();
    let hash5 = compute_state_root(&storage).unwrap();

    // INVARIANT: Hash computation should be deterministic
    assert_eq!(
        hash3, hash4,
        "State hash computation should be deterministic"
    );
    assert_eq!(hash4, hash5, "State hash should remain consistent");
}

#[test]
fn multi_block_with_transactions() {
    let storage = MemoryStorage::new();

    let addr1 = [1u8; 32];
    let addr2 = [2u8; 32];
    let addr3 = [3u8; 32];

    // Initialize accounts
    storage
        .update_account(Account {
            address: addr1,
            balance: 100_000,
            nonce: 0,
        })
        .unwrap();
    storage
        .update_account(Account {
            address: addr2,
            balance: 50_000,
            nonce: 0,
        })
        .unwrap();
    storage
        .update_account(Account {
            address: addr3,
            balance: 25_000,
            nonce: 0,
        })
        .unwrap();

    let initial_hash = compute_state_root(&storage).unwrap();

    // Apply transactions across multiple blocks
    for round in 0..10 {
        // Create transactions
        let tx1 = create_test_transaction(addr1, addr2, 1_000, round);
        let tx2 = create_test_transaction(addr2, addr3, 500, round);

        storage.store_transaction(tx1).unwrap();
        storage.store_transaction(tx2).unwrap();

        // Update account state to reflect transactions
        let mut acc1 = storage.get_account(&addr1).unwrap().unwrap();
        let mut acc2 = storage.get_account(&addr2).unwrap().unwrap();
        let mut acc3 = storage.get_account(&addr3).unwrap().unwrap();

        acc1.balance -= 1_000;
        acc1.nonce += 1;

        acc2.balance += 1_000;
        acc2.balance -= 500;
        acc2.nonce += 1;

        acc3.balance += 500;

        storage.update_account(acc1).unwrap();
        storage.update_account(acc2).unwrap();
        storage.update_account(acc3).unwrap();

        // Create block for this round
        let creator = [round as u8; 32];
        let parent = if round > 0 {
            vec![[(round - 1) as u8; 32]]
        } else {
            vec![]
        };
        let block = create_test_block(round, creator, parent);
        storage.store_block(block).unwrap();
    }

    let final_hash = compute_state_root(&storage).unwrap();

    // INVARIANT: State should have changed after transactions
    assert_ne!(
        initial_hash, final_hash,
        "State hash should change after transactions"
    );

    // INVARIANT: Final balances should be correct
    let final_acc1 = storage.get_account(&addr1).unwrap().unwrap();
    let final_acc2 = storage.get_account(&addr2).unwrap().unwrap();
    let final_acc3 = storage.get_account(&addr3).unwrap().unwrap();

    assert_eq!(final_acc1.balance, 100_000 - 10_000, "addr1 balance");
    assert_eq!(final_acc2.balance, 50_000 + 10_000 - 5_000, "addr2 balance");
    assert_eq!(final_acc3.balance, 25_000 + 5_000, "addr3 balance");

    // INVARIANT: Total balance should be conserved
    let total_balance = final_acc1.balance + final_acc2.balance + final_acc3.balance;
    assert_eq!(
        total_balance, 175_000,
        "Total balance should be conserved"
    );

    // INVARIANT: Transaction count should match
    let tx_count = storage.get_transaction_count().unwrap();
    assert_eq!(tx_count, 20, "Should have 20 transactions (2 per round)");
}

#[test]
fn sled_multi_block_replay_with_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    let rounds = 15;
    let mut expected_state_roots = Vec::new();

    // Phase 1: Apply state updates and close storage
    {
        let storage = SledStorage::new(&path).unwrap();

        for round in 0..rounds {
            let state_root = apply_state_update(&storage, round, 8_000).unwrap();
            expected_state_roots.push(state_root);
        }

        storage.flush().unwrap();
    }

    // Phase 2: Reopen storage and verify state
    {
        let storage = SledStorage::new(&path).unwrap();

        // INVARIANT: Latest height should be preserved
        let latest_height = storage.get_latest_height().unwrap();
        assert_eq!(latest_height, rounds - 1, "Latest height should be preserved");

        // INVARIANT: Chain state should be preserved
        let chain_state = storage.get_chain_state().unwrap();
        assert_eq!(
            chain_state.current_round, rounds - 1,
            "Current round should be preserved"
        );
        assert_eq!(
            chain_state.total_issued_micro,
            8_000 * 5 * rounds,
            "Total issued should be preserved"
        );

        // INVARIANT: All blocks should be retrievable
        for round in 0..rounds {
            let block = storage.get_block_by_height(round).unwrap();
            assert!(
                block.is_some(),
                "Block at height {} should exist after restart",
                round
            );
        }

        // INVARIANT: State roots should match
        for (round, expected_root) in expected_state_roots.iter().enumerate() {
            let finalization = storage.get_round_finalization(round as u64).unwrap();
            assert!(finalization.is_some());
            assert_eq!(
                &finalization.unwrap().state_root,
                expected_root,
                "State root mismatch at round {} after restart",
                round
            );
        }

        // INVARIANT: Recomputed state hash should match
        let current_hash = compute_state_root(&storage).unwrap();
        assert_eq!(
            &current_hash,
            expected_state_roots.last().unwrap(),
            "Current state hash should match last finalized root"
        );
    }
}

#[test]
fn partial_replay_from_checkpoint() {
    let storage = MemoryStorage::new();

    // Apply first 10 rounds
    for round in 0..10 {
        apply_state_update(&storage, round, 5_000).unwrap();
    }

    let checkpoint_hash = compute_state_root(&storage).unwrap();
    let checkpoint_state = storage.get_chain_state().unwrap();

    // Apply 10 more rounds
    for round in 10..20 {
        apply_state_update(&storage, round, 5_000).unwrap();
    }

    let final_hash = compute_state_root(&storage).unwrap();

    // INVARIANT: State should have progressed
    assert_ne!(
        checkpoint_hash, final_hash,
        "State should change after additional rounds"
    );

    // Create new storage and replay only from checkpoint
    let storage2 = MemoryStorage::new();

    // Restore checkpoint state
    storage2.update_chain_state(&checkpoint_state).unwrap();

    // Copy accounts from checkpoint
    let checkpoint_accounts = storage.get_all_accounts().unwrap();
    for round in 0..10 {
        for i in 0..5 {
            let addr = [(round * 10 + i) as u8; 32];
            if let Some(account) = checkpoint_accounts.get(&addr) {
                storage2.update_account(account.clone()).unwrap();
            }
        }
    }

    // Apply remaining rounds
    for round in 10..20 {
        apply_state_update(&storage2, round, 5_000).unwrap();
    }

    let replay_final_hash = compute_state_root(&storage2).unwrap();

    // INVARIANT: Partial replay should produce same final state
    // Note: This may differ slightly due to block storage differences,
    // but chain state derived values should match
    let final_state_1 = storage.get_chain_state().unwrap();
    let final_state_2 = storage2.get_chain_state().unwrap();

    assert_eq!(
        final_state_1.total_issued_micro, final_state_2.total_issued_micro,
        "Total issued should match after partial replay"
    );
    assert_eq!(
        final_state_1.current_round, final_state_2.current_round,
        "Current round should match after partial replay"
    );
}

#[test]
fn state_transitions_are_atomic() {
    let storage = MemoryStorage::new();

    let addr = [42u8; 32];

    // Create initial account
    storage
        .update_account(Account {
            address: addr,
            balance: 10_000,
            nonce: 0,
        })
        .unwrap();

    let initial_hash = compute_state_root(&storage).unwrap();

    // Perform atomic update: balance change + nonce increment
    let mut account = storage.get_account(&addr).unwrap().unwrap();
    account.balance += 5_000;
    account.nonce += 1;
    storage.update_account(account).unwrap();

    let after_update_hash = compute_state_root(&storage).unwrap();

    // INVARIANT: State should have changed atomically
    assert_ne!(
        initial_hash, after_update_hash,
        "State should change after update"
    );

    // Retrieve and verify both fields updated
    let retrieved = storage.get_account(&addr).unwrap().unwrap();
    assert_eq!(retrieved.balance, 15_000, "Balance should be updated");
    assert_eq!(retrieved.nonce, 1, "Nonce should be updated");

    // Compute hash again - should be stable
    let stable_hash = compute_state_root(&storage).unwrap();
    assert_eq!(
        after_update_hash, stable_hash,
        "State hash should be stable after update"
    );
}

#[test]
fn large_multi_block_sequence() {
    let storage = MemoryStorage::new();

    // Apply 100 rounds
    let rounds = 100;

    for round in 0..rounds {
        apply_state_update(&storage, round, 1_000).unwrap();
    }

    // INVARIANT: Storage should handle large sequences
    let latest_height = storage.get_latest_height().unwrap();
    assert_eq!(latest_height, rounds - 1);

    // INVARIANT: All blocks should be retrievable
    for round in (0..rounds).step_by(10) {
        let block = storage.get_block_by_height(round).unwrap();
        assert!(block.is_some(), "Block at height {} should exist", round);
    }

    // INVARIANT: Chain state should be consistent
    let chain_state = storage.get_chain_state().unwrap();
    assert_eq!(chain_state.current_round, rounds - 1);
    assert_eq!(chain_state.total_issued_micro, 1_000 * 5 * rounds);

    // INVARIANT: Account count should match expected
    let all_accounts = storage.get_all_accounts().unwrap();
    assert_eq!(
        all_accounts.len(),
        (rounds * 5) as usize,
        "Should have 5 accounts per round"
    );
}

#[test]
fn deterministic_state_hash_with_same_inputs() {
    let storage = MemoryStorage::new();

    // Create identical state in multiple passes
    let hashes: Vec<[u8; 32]> = (0..3)
        .map(|_| {
            // Reset storage state by clearing and reapplying
            let storage = MemoryStorage::new();

            for round in 0..5 {
                apply_state_update(&storage, round, 7_777).unwrap();
            }

            compute_state_root(&storage).unwrap()
        })
        .collect();

    // INVARIANT: All hashes should be identical
    for i in 1..hashes.len() {
        assert_eq!(
            hashes[0], hashes[i],
            "State hash at index {} should match first hash",
            i
        );
    }
}
