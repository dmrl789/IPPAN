// Minimal placeholder to satisfy bench target during Docker builds
// Replace with real benchmarks as needed.

fn main() {}

#![feature(test)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan::{
    consensus::{ConsensusEngine, ConsensusConfig, Block, Transaction, HashTimer},
    utils::crypto,
};
use std::time::Instant;

/// Benchmark HashTimer creation and validation
fn benchmark_hashtimer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("HashTimer Operations");
    
    group.bench_function("create_hashtimer", |b| {
        b.iter(|| {
            let hashtimer = HashTimer::with_ippan_time(
                black_box([0u8; 32]),
                black_box([1u8; 32]),
                black_box(1234567890),
            );
            black_box(hashtimer)
        });
    });
    
    group.bench_function("validate_hashtimer", |b| {
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        b.iter(|| {
            black_box(hashtimer.is_valid(10))
        });
    });
    
    group.bench_function("ippan_time_calculation", |b| {
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        b.iter(|| {
            black_box(hashtimer.ippan_time_ns)
        });
    });
    
    group.finish();
}

/// Benchmark block creation and validation
fn benchmark_block_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Block Operations");
    
    let config = ConsensusConfig::default();
    let mut consensus = ConsensusEngine::new(config).unwrap();
    
    group.bench_function("create_block_single_tx", |b| {
        b.iter(|| {
            let hashtimer = HashTimer::with_ippan_time(
                [0u8; 32],
                [1u8; 32],
                1234567890,
            );
            let transactions = vec![
                Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
            ];
            let block = consensus.create_block(transactions, [4u8; 32]).unwrap();
            black_box(block)
        });
    });
    
    group.bench_function("create_block_multiple_tx", |b| {
        b.iter(|| {
            let hashtimer = HashTimer::with_ippan_time(
                [0u8; 32],
                [1u8; 32],
                1234567890,
            );
            let transactions = (0..10).map(|i| {
                Transaction::new(
                    [i as u8; 32],
                    1000 + i as u64,
                    [i as u8; 32],
                    hashtimer.clone(),
                )
            }).collect();
            let block = consensus.create_block(transactions, [4u8; 32]).unwrap();
            black_box(block)
        });
    });
    
    group.bench_function("validate_block", |b| {
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        let transactions = vec![
            Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
        ];
        let block = Block::new(1, transactions, [4u8; 32], hashtimer);
        
        b.iter(|| {
            black_box(consensus.validate_block(&block).unwrap())
        });
    });
    
    group.finish();
}

/// Benchmark consensus engine operations
fn benchmark_consensus_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("Consensus Engine");
    
    group.bench_function("add_validator", |b| {
        let config = ConsensusConfig::default();
        let mut consensus = ConsensusEngine::new(config).unwrap();
        
        b.iter(|| {
            let node_id = crypto::generate_node_id();
            black_box(consensus.add_validator(node_id, 1000).unwrap())
        });
    });
    
    group.bench_function("get_validators", |b| {
        let config = ConsensusConfig::default();
        let mut consensus = ConsensusEngine::new(config).unwrap();
        consensus.add_validator([1u8; 32], 1000).unwrap();
        consensus.add_validator([2u8; 32], 2000).unwrap();
        
        b.iter(|| {
            black_box(consensus.get_validators())
        });
    });
    
    group.bench_function("get_ippan_time", |b| {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        
        b.iter(|| {
            black_box(consensus.get_ippan_time())
        });
    });
    
    group.bench_function("current_round", |b| {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        
        b.iter(|| {
            black_box(consensus.current_round())
        });
    });
    
    group.finish();
}

/// Benchmark transaction operations
fn benchmark_transaction_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Transaction Operations");
    
    group.bench_function("create_transaction", |b| {
        b.iter(|| {
            let hashtimer = HashTimer::with_ippan_time(
                [0u8; 32],
                [1u8; 32],
                1234567890,
            );
            let tx = Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer);
            black_box(tx)
        });
    });
    
    group.bench_function("validate_transaction", |b| {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        let tx = Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer);
        
        b.iter(|| {
            black_box(consensus.validate_transaction(&tx).unwrap())
        });
    });
    
    group.bench_function("transaction_hash", |b| {
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        let tx = Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer);
        
        b.iter(|| {
            black_box(tx.hash())
        });
    });
    
    group.finish();
}

/// Benchmark block hash calculation
fn benchmark_block_hash_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Block Hash Calculation");
    
    group.bench_function("calculate_block_hash", |b| {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        let transactions = vec![
            Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
        ];
        
        b.iter(|| {
            black_box(consensus.calculate_block_hash(&transactions, 1, [4u8; 32]))
        });
    });
    
    group.bench_function("block_hash_verification", |b| {
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        let transactions = vec![
            Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
        ];
        let block = Block::new(1, transactions, [4u8; 32], hashtimer);
        
        b.iter(|| {
            black_box(block.header.hash)
        });
    });
    
    group.finish();
}

/// Benchmark consensus throughput
fn benchmark_consensus_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Consensus Throughput");
    
    group.bench_function("create_blocks_sequential", |b| {
        let config = ConsensusConfig::default();
        let mut consensus = ConsensusEngine::new(config).unwrap();
        
        b.iter(|| {
            for i in 0..10 {
                let hashtimer = HashTimer::with_ippan_time(
                    [i as u8; 32],
                    [1u8; 32],
                    1234567890 + i as u64,
                );
                let transactions = vec![
                    Transaction::new([i as u8; 32], 1000, [3u8; 32], hashtimer.clone()),
                ];
                let block = consensus.create_block(transactions, [4u8; 32]).unwrap();
                consensus.add_block(block).unwrap();
            }
        });
    });
    
    group.bench_function("validate_blocks_batch", |b| {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        let mut blocks = Vec::new();
        
        for i in 0..10 {
            let hashtimer = HashTimer::with_ippan_time(
                [i as u8; 32],
                [1u8; 32],
                1234567890 + i as u64,
            );
            let transactions = vec![
                Transaction::new([i as u8; 32], 1000, [3u8; 32], hashtimer.clone()),
            ];
            let block = Block::new(i, transactions, [4u8; 32], hashtimer);
            blocks.push(block);
        }
        
        b.iter(|| {
            for block in &blocks {
                black_box(consensus.validate_block(block).unwrap());
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    consensus_benches,
    benchmark_hashtimer_operations,
    benchmark_block_operations,
    benchmark_consensus_engine,
    benchmark_transaction_operations,
    benchmark_block_hash_calculation,
    benchmark_consensus_throughput
);
criterion_main!(consensus_benches); 