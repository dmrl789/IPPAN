use ippan_storage::{Account, MemoryStorage, SledStorage, Storage};
use ippan_types::{Amount, Block, ChainState, Transaction};
use tempfile::TempDir;

fn create_payment(from: [u8; 32], to: [u8; 32], amount: u64, nonce: u64) -> Transaction {
    let mut tx = Transaction::new(from, to, Amount::from_atomic(amount as u128), nonce);
    tx.refresh_id();
    tx
}

fn create_block(round: u64, parents: Vec<[u8; 32]>, txs: Vec<Transaction>) -> Block {
    Block::new(parents, txs, round, [round as u8; 32])
}

fn apply_accounts_for_tx<S: Storage>(storage: &S, tx: &Transaction) {
    let mut sender = storage.get_account(&tx.from).unwrap().unwrap_or(Account {
        address: tx.from,
        balance: 0,
        nonce: 0,
    });
    let mut recipient = storage.get_account(&tx.to).unwrap().unwrap_or(Account {
        address: tx.to,
        balance: 0,
        nonce: 0,
    });

    let amount = tx.amount.atomic() as u64;
    sender.balance = sender.balance.saturating_sub(amount);
    sender.nonce = sender.nonce.saturating_add(1);
    recipient.balance = recipient.balance.saturating_add(amount);

    storage.update_account(sender).unwrap();
    storage.update_account(recipient).unwrap();
}

fn seed_accounts<S: Storage>(storage: &S) {
    let accounts = vec![
        Account {
            address: [1u8; 32],
            balance: 5_000,
            nonce: 0,
        },
        Account {
            address: [2u8; 32],
            balance: 2_000,
            nonce: 0,
        },
        Account {
            address: [3u8; 32],
            balance: 0,
            nonce: 0,
        },
    ];

    for account in accounts {
        storage.update_account(account).unwrap();
    }
}

#[test]
fn sled_restart_restores_canonical_and_fork_branches() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().to_path_buf();

    let (block_one, block_two_canon, block_two_fork, expected_accounts, chain_state_root) = {
        let storage = SledStorage::new(&path).expect("sled storage");
        storage.initialize().expect("init");
        seed_accounts(&storage);

        let tx1 = create_payment([1u8; 32], [2u8; 32], 500, 1);
        let tx2_canon = create_payment([2u8; 32], [3u8; 32], 700, 2);
        let tx2_fork = create_payment([1u8; 32], [3u8; 32], 300, 2);

        let block1 = create_block(1, vec![], vec![tx1.clone()]);
        apply_accounts_for_tx(&storage, &tx1);
        storage.store_transaction(tx1.clone()).unwrap();
        storage.store_block(block1.clone()).unwrap();

        let block2_canon = create_block(2, vec![block1.header.id], vec![tx2_canon.clone()]);
        apply_accounts_for_tx(&storage, &tx2_canon);
        storage.store_transaction(tx2_canon.clone()).unwrap();
        storage.store_block(block2_canon.clone()).unwrap();

        let block2_fork = create_block(2, vec![block1.header.id], vec![tx2_fork.clone()]);
        storage.store_transaction(tx2_fork.clone()).unwrap();
        storage.store_block(block2_fork.clone()).unwrap();

        let mut chain_state = ChainState::new();
        chain_state.set_height(2);
        chain_state.set_round(2);
        chain_state.set_state_root(block2_canon.header.id);
        chain_state.set_last_updated(block2_canon.header.hashtimer.timestamp_us as u64);
        chain_state.add_issued_micro(42);
        storage.update_chain_state(&chain_state).unwrap();
        storage.flush().unwrap();

        let expected_accounts = vec![
            (storage.get_account(&[1u8; 32]).unwrap().unwrap().balance),
            (storage.get_account(&[2u8; 32]).unwrap().unwrap().balance),
            (storage.get_account(&[3u8; 32]).unwrap().unwrap().balance),
        ];

        (
            block1,
            block2_canon,
            block2_fork,
            expected_accounts,
            chain_state.state_root,
        )
    };

    let reopened = SledStorage::new(&path).expect("reopen storage");

    let latest_height = reopened.get_latest_height().unwrap();
    assert_eq!(latest_height, 2);

    let stored_block1 = reopened.get_block(&block_one.hash()).unwrap();
    assert!(stored_block1.is_some());

    let stored_canon = reopened.get_block(&block_two_canon.hash()).unwrap();
    assert!(stored_canon.is_some());

    let stored_fork = reopened.get_block(&block_two_fork.hash()).unwrap();
    assert!(
        stored_fork.is_some(),
        "fork block should be retained for audit"
    );

    let reopened_state = reopened.get_chain_state().unwrap();
    assert_eq!(reopened_state.current_height, 2);
    assert_eq!(reopened_state.current_round, 2);
    assert_eq!(reopened_state.state_root, chain_state_root);
    assert_eq!(reopened_state.total_issued_micro, 42);

    let reopened_balances: Vec<u64> = vec![
        reopened.get_account(&[1u8; 32]).unwrap().unwrap().balance,
        reopened.get_account(&[2u8; 32]).unwrap().unwrap().balance,
        reopened.get_account(&[3u8; 32]).unwrap().unwrap().balance,
    ];
    assert_eq!(reopened_balances, expected_accounts);
}

#[test]
fn sled_persists_unflushed_writes_after_restart() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().to_path_buf();

    let block2;
    {
        let storage = SledStorage::new(&path).expect("sled storage");
        storage.initialize().expect("init");
        seed_accounts(&storage);

        let tx1 = create_payment([1u8; 32], [2u8; 32], 250, 1);
        let block1 = create_block(1, vec![], vec![tx1.clone()]);
        apply_accounts_for_tx(&storage, &tx1);
        storage.store_transaction(tx1).unwrap();
        storage.store_block(block1.clone()).unwrap();
        let mut state = ChainState::new();
        state.set_height(1);
        state.set_round(1);
        state.set_state_root(block1.header.id);
        storage.update_chain_state(&state).unwrap();
        storage.flush().unwrap();

        let tx2 = create_payment([2u8; 32], [3u8; 32], 125, 2);
        block2 = create_block(2, vec![block1.header.id], vec![tx2.clone()]);
        apply_accounts_for_tx(&storage, &tx2);
        storage.store_transaction(tx2).unwrap();
        storage.store_block(block2.clone()).unwrap();

        let mut state_after = state.clone();
        state_after.set_height(2);
        state_after.set_round(2);
        state_after.set_state_root(block2.header.id);
        storage.update_chain_state(&state_after).unwrap();
        // intentionally skip flush to mimic abrupt stop
    }

    let reopened = SledStorage::new(&path).expect("reopen storage");
    assert_eq!(reopened.get_latest_height().unwrap(), 2);

    let stored_block2 = reopened.get_block(&block2.hash()).unwrap();
    assert!(stored_block2.is_some());

    let reopened_state = reopened.get_chain_state().unwrap();
    assert_eq!(reopened_state.current_height, 2);
    assert_eq!(reopened_state.state_root, block2.header.id);
}

#[test]
fn reorg_switches_canonical_branch_without_double_applying_state() {
    let storage = MemoryStorage::new();
    seed_accounts(&storage);

    let tx1 = create_payment([1u8; 32], [2u8; 32], 200, 1);
    let block1 = create_block(1, vec![], vec![tx1.clone()]);
    apply_accounts_for_tx(&storage, &tx1);
    storage.store_transaction(tx1).unwrap();
    storage.store_block(block1.clone()).unwrap();

    let base_after_block1 = vec![
        storage.get_account(&[1u8; 32]).unwrap().unwrap(),
        storage.get_account(&[2u8; 32]).unwrap().unwrap(),
        storage.get_account(&[3u8; 32]).unwrap().unwrap(),
    ];

    let tx_canon = create_payment([2u8; 32], [3u8; 32], 75, 2);
    let tx_fork = create_payment([2u8; 32], [1u8; 32], 50, 2);

    let mut canon_accounts = base_after_block1.clone();
    canon_accounts[1].balance = canon_accounts[1].balance.saturating_sub(75);
    canon_accounts[1].nonce = canon_accounts[1].nonce.saturating_add(1);
    canon_accounts[2].balance = canon_accounts[2].balance.saturating_add(75);

    for account in &canon_accounts {
        storage.update_account(account.clone()).unwrap();
    }

    let block2_canon = create_block(2, vec![block1.header.id], vec![tx_canon.clone()]);
    storage.store_transaction(tx_canon).unwrap();
    storage.store_block(block2_canon.clone()).unwrap();

    let mut chain_state = ChainState::new();
    chain_state.set_height(2);
    chain_state.set_round(2);
    chain_state.set_state_root(block2_canon.header.id);
    storage.update_chain_state(&chain_state).unwrap();

    let mut fork_accounts = base_after_block1;
    fork_accounts[1].balance = fork_accounts[1].balance.saturating_sub(50);
    fork_accounts[1].nonce = fork_accounts[1].nonce.saturating_add(1);
    fork_accounts[0].balance = fork_accounts[0].balance.saturating_add(50);

    let block2_fork = create_block(2, vec![block1.header.id], vec![tx_fork.clone()]);
    storage.store_transaction(tx_fork).unwrap();
    storage.store_block(block2_fork.clone()).unwrap();

    for account in &fork_accounts {
        storage.update_account(account.clone()).unwrap();
    }

    let mut chain_state_after_reorg = chain_state.clone();
    chain_state_after_reorg.set_state_root(block2_fork.header.id);
    storage
        .update_chain_state(&chain_state_after_reorg)
        .unwrap();

    let total_balance: u64 = fork_accounts.iter().map(|a| a.balance).sum();
    let reopened_accounts: Vec<Account> = vec![
        storage.get_account(&[1u8; 32]).unwrap().unwrap(),
        storage.get_account(&[2u8; 32]).unwrap().unwrap(),
        storage.get_account(&[3u8; 32]).unwrap().unwrap(),
    ];
    let reopened_total: u64 = reopened_accounts.iter().map(|a| a.balance).sum();

    assert_eq!(reopened_total, total_balance);
    assert_eq!(reopened_accounts[0].balance, fork_accounts[0].balance);
    assert_eq!(reopened_accounts[1].balance, fork_accounts[1].balance);
    assert_eq!(reopened_accounts[2].balance, fork_accounts[2].balance);
    assert_eq!(
        storage.get_chain_state().unwrap().state_root,
        block2_fork.header.id
    );
    assert!(storage.get_block(&block2_canon.hash()).unwrap().is_some());
}
