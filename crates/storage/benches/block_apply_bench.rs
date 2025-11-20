use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use ippan_storage::{MemoryStorage, Storage};
use ippan_types::currency::Amount;
use ippan_types::round::{RoundCertificate, RoundFinalizationRecord, RoundWindow};
use ippan_types::{Block, IppanTimeMicros, Transaction};

fn synthetic_transactions(count: usize, from: [u8; 32], to: [u8; 32]) -> Vec<Transaction> {
    (0..count)
        .map(|nonce| Transaction::new(from, to, Amount::from_micro_ipn(1_000), nonce as u64))
        .collect()
}

fn synthetic_block(tx_count: usize, round: u64) -> Block {
    let parent: [u8; 32] = [0u8; 32];
    let creator: [u8; 32] = [1u8; 32];
    let transactions = synthetic_transactions(tx_count, creator, [2u8; 32]);
    Block::with_parent(parent, transactions, round, creator)
}

fn apply_block(storage: &MemoryStorage, block: Block) {
    storage.store_block(block.clone()).expect("store block");
    for tx in block.transactions.iter().cloned() {
        storage
            .store_transaction(tx.clone())
            .expect("store transaction");
    }

    let mut chain_state = storage.get_chain_state().unwrap_or_default();
    chain_state.set_height(block.header.round);
    chain_state.set_round(block.header.round);
    chain_state.set_last_updated(block.header.hashtimer.timestamp_us as u64);
    storage
        .update_chain_state(&chain_state)
        .expect("update chain state");

    let round = block.header.round;
    let window_time = IppanTimeMicros(block.header.hashtimer.timestamp_us as u64);
    let window = RoundWindow {
        id: round,
        start_us: window_time,
        end_us: window_time,
    };
    let proof = RoundCertificate {
        round,
        block_ids: vec![block.header.id],
        agg_sig: Vec::new(),
    };
    let ordered_tx_ids: Vec<[u8; 32]> = block.transactions.iter().map(Transaction::hash).collect();

    let finalization = RoundFinalizationRecord {
        round,
        window,
        ordered_tx_ids,
        fork_drops: Vec::new(),
        state_root: [0u8; 32],
        proof,
        total_fees_atomic: None,
        treasury_fees_atomic: None,
        applied_payments: Some(block.transactions.len() as u64),
        rejected_payments: Some(0),
    };

    storage
        .store_round_finalization(finalization)
        .expect("store round finalization");
}

fn bench_block_apply(c: &mut Criterion) {
    c.bench_function("memory_storage_block_apply", |b| {
        b.iter_batched(
            || (MemoryStorage::new(), synthetic_block(64, 1)),
            |(storage, block)| apply_block(&storage, block),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(storage_benches, bench_block_apply);
criterion_main!(storage_benches);
