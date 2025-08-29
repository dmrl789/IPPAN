use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan_common::{KeyPair, Transaction, crypto::hashtimer, time::ippan_time_us, merkle::compute_merkle_root};
use std::time::Instant;

/// Benchmark Ed25519 batch verification with varying batch sizes
fn bench_ed25519_batch_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519_batch_verify");
    
    for batch_size in [10, 50, 100, 500, 1000] {
        group.bench_function(&format!("batch_size_{}", batch_size), |b| {
            // Generate test data
            let keypairs: Vec<KeyPair> = (0..batch_size)
                .map(|_| KeyPair::generate())
                .collect();
            
            let messages: Vec<Vec<u8>> = (0..batch_size)
                .map(|i| format!("message_{}", i).into_bytes())
                .collect();
            
            let signatures: Vec<_> = keypairs.iter()
                .zip(messages.iter())
                .map(|(kp, msg)| kp.sign(msg).unwrap())
                .collect();
            
            let public_keys: Vec<_> = keypairs.iter()
                .map(|kp| kp.public_key)
                .collect();
            
            let message_refs: Vec<&[u8]> = messages.iter()
                .map(|m| m.as_slice())
                .collect();
            
            b.iter(|| {
                black_box(KeyPair::batch_verify(
                    &public_keys,
                    &message_refs,
                    &signatures
                ).unwrap());
            });
        });
    }
    
    group.finish();
}

/// Benchmark mempool enqueue throughput
fn bench_mempool_enqueue(c: &mut Criterion) {
    let mut group = c.benchmark_group("mempool_enqueue");
    
    for shard_count in [1, 4, 8, 16] {
        group.bench_function(&format!("shards_{}", shard_count), |b| {
            use ippan_node::mempool::Mempool;
            
            let mut mempool = Mempool::new(shard_count);
            
            b.iter(|| {
                let keypair = KeyPair::generate();
                let ippan_time = ippan_time_us();
                let tx_id = [0u8; 32]; // Placeholder
                let hashtimer = hashtimer(&tx_id);
                
                let tx = Transaction {
                    ver: 1,
                    from_pub: keypair.public_key,
                    to_addr: [0u8; 32], // Placeholder
                    amount: 1000,
                    nonce: 0,
                    ippan_time_us: ippan_time,
                    hashtimer,
                    sig: [0u8; 64], // Placeholder
                };
                
                black_box(mempool.add_transaction(tx));
            });
        });
    }
    
    group.finish();
}

/// Benchmark block building throughput
fn bench_block_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_build");
    
    for tx_count in [100, 500, 1000, 2000] {
        group.bench_function(&format!("tx_count_{}", tx_count), |b| {
            use ippan_node::block::BlockBuilder;
            
            let mut builder = BlockBuilder::new();
            
            // Generate test transactions
            let transactions: Vec<Transaction> = (0..tx_count)
                .map(|i| {
                    let keypair = KeyPair::generate();
                    let ippan_time = ippan_time_us();
                    let tx_id = [0u8; 32]; // Placeholder
                    let hashtimer = hashtimer(&tx_id);
                    
                    Transaction {
                        ver: 1,
                        from_pub: keypair.public_key,
                        to_addr: [0u8; 32], // Placeholder
                        amount: 1000,
                        nonce: i as u64,
                        ippan_time_us: ippan_time,
                        hashtimer,
                        sig: [0u8; 64], // Placeholder
                    }
                })
                .collect();
            
            b.iter(|| {
                black_box(builder.build_block(&transactions));
            });
        });
    }
    
    group.finish();
}

/// Benchmark merkle tree computation
fn bench_merkle_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_compute");
    
    for tx_count in [100, 500, 1000, 2000, 5000] {
        group.bench_function(&format!("tx_count_{}", tx_count), |b| {
            // Generate test transaction IDs
            let tx_ids: Vec<_> = (0..tx_count)
                .map(|i| {
                    let mut hash = [0u8; 32];
                    hash[0] = (i % 256) as u8;
                    hash
                })
                .collect();
            
            b.iter(|| {
                black_box(compute_merkle_root(&tx_ids).unwrap());
            });
        });
    }
    
    group.finish();
}

/// Benchmark transaction serialization/deserialization
fn bench_transaction_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_serialization");
    
    // Generate test transaction
    let keypair = KeyPair::generate();
    let ippan_time = ippan_time_us();
    let tx_id = [0u8; 32]; // Placeholder
    let hashtimer = hashtimer(&tx_id);
    
    let tx = Transaction {
        ver: 1,
        from_pub: keypair.public_key,
        to_addr: [0u8; 32], // Placeholder
        amount: 1000,
        nonce: 0,
        ippan_time_us: ippan_time,
        hashtimer,
        sig: [0u8; 64], // Placeholder
    };
    
    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&tx).unwrap());
        });
    });
    
    let tx_data = bincode::serialize(&tx).unwrap();
    
    group.bench_function("deserialize", |b| {
        b.iter(|| {
            black_box(bincode::deserialize::<Transaction>(&tx_data).unwrap());
        });
    });
    
    group.finish();
}

/// Benchmark HashTimer computation
fn bench_hashtimer(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashtimer");
    
    let tx_id = [0u8; 32];
    
    group.bench_function("compute", |b| {
        b.iter(|| {
            black_box(hashtimer(&tx_id));
        });
    });
    
    group.finish();
}

/// Benchmark IPPAN time computation
fn bench_ippan_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("ippan_time");
    
    group.bench_function("get_time", |b| {
        b.iter(|| {
            black_box(ippan_time_us());
        });
    });
    
    group.finish();
}

/// Benchmark address derivation
fn bench_address_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("address_derivation");
    
    let keypair = KeyPair::generate();
    
    group.bench_function("derive", |b| {
        b.iter(|| {
            black_box(ippan_common::crypto::derive_address(&keypair.public_key));
        });
    });
    
    group.finish();
}

/// Custom benchmark for end-to-end transaction processing
fn bench_e2e_transaction_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_transaction_processing");
    
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            // Generate transaction
            let keypair = KeyPair::generate();
            let ippan_time = ippan_time_us();
            let tx_id = [0u8; 32]; // Placeholder
            let hashtimer = hashtimer(&tx_id);
            
            let mut tx = Transaction {
                ver: 1,
                from_pub: keypair.public_key,
                to_addr: [0u8; 32], // Placeholder
                amount: 1000,
                nonce: 0,
                ippan_time_us: ippan_time,
                hashtimer,
                sig: [0u8; 64], // Will be set after signing
            };
            
            // Sign transaction
            tx.sig = keypair.sign(&bincode::serialize(&tx).unwrap()).unwrap();
            
            // Serialize
            let tx_data = bincode::serialize(&tx).unwrap();
            
            // Verify signature
            let is_valid = tx.verify().unwrap();
            
            black_box((tx_data, is_valid));
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_ed25519_batch_verify,
    bench_mempool_enqueue,
    bench_block_build,
    bench_merkle_compute,
    bench_transaction_serialization,
    bench_hashtimer,
    bench_ippan_time,
    bench_address_derivation,
    bench_e2e_transaction_processing,
);

criterion_main!(benches);
