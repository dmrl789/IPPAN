use ippan_storage::{export_snapshot, import_snapshot, MemoryStorage, Storage, StorageLike};
use ippan_types::{Amount, Block, ChainState, Transaction};
use tempfile::TempDir;

fn build_linear_chain(len: u64) -> (Vec<Block>, Vec<Transaction>) {
    let mut blocks = Vec::new();
    let mut txs = Vec::new();
    let mut parent_ids: Vec<[u8; 32]> = Vec::new();

    for round in 1..=len {
        let mut tx = Transaction::new(
            [round as u8; 32],
            [round as u8 + 1; 32],
            Amount::from_atomic(1_000u128 + round as u128),
            round,
        );
        tx.refresh_id();
        txs.push(tx.clone());

        let block = Block::new(parent_ids.clone(), vec![tx], round, [round as u8; 32]);

        parent_ids = vec![block.header.id];
        blocks.push(block);
    }

    (blocks, txs)
}

fn apply_chain(storage: &impl Storage, blocks: &[Block], txs: &[Transaction]) -> ChainState {
    let mut chain_state = ChainState::new();
    for (idx, block) in blocks.iter().enumerate() {
        let tx = &txs[idx];
        storage.store_transaction(tx.clone()).unwrap();
        storage.store_block(block.clone()).unwrap();

        chain_state.set_height(block.header.round);
        chain_state.set_round(block.header.round);
        chain_state.set_state_root(block.header.id);
        chain_state.set_last_updated(block.header.hashtimer.timestamp_us as u64);
        chain_state.add_issued_micro(1_000_000 + (block.header.round as u128));
    }
    storage.update_chain_state(&chain_state).unwrap();
    chain_state
}

#[test]
fn snapshot_replay_roundtrip_preserves_state() {
    let (blocks, txs) = build_linear_chain(6);

    let storage = MemoryStorage::new();
    let chain_state = apply_chain(&storage, &blocks, &txs);

    let snapshot_dir = TempDir::new().expect("snapshot dir");
    let manifest = export_snapshot(&storage, snapshot_dir.path()).expect("export snapshot");

    let mut replay_storage = MemoryStorage::new();
    let imported_manifest = import_snapshot(&mut replay_storage, snapshot_dir.path())
        .expect("import snapshot");

    assert_eq!(manifest.height, imported_manifest.height);
    assert_eq!(manifest.blocks_count, imported_manifest.blocks_count);

    let replay_state = replay_storage.snapshot_chain_state().expect("chain state");
    assert_eq!(replay_state.current_height, chain_state.current_height);
    assert_eq!(replay_state.current_round, chain_state.current_round);
    assert_eq!(replay_state.state_root, chain_state.state_root);
    assert_eq!(replay_state.last_updated, chain_state.last_updated);

    let replay_blocks = replay_storage.snapshot_blocks().expect("replay blocks");
    assert_eq!(replay_blocks.len(), blocks.len());

    let second_storage = MemoryStorage::new();
    let second_state = apply_chain(&second_storage, &blocks, &txs);

    let replay_block_ids: Vec<_> = replay_blocks.iter().map(|b| b.header.id).collect();
    let second_block_ids: Vec<_> = second_storage
        .snapshot_blocks()
        .unwrap()
        .into_iter()
        .map(|b| b.header.id)
        .collect();

    assert_eq!(
        replay_block_ids, second_block_ids,
        "Replay from snapshot must match deterministic re-application",
    );
    assert_eq!(
        replay_storage.snapshot_chain_state().unwrap().state_root,
        second_state.state_root,
        "State root should remain consistent after replay",
    );
}
