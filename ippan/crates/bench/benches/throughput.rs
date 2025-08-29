use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan_common::{
    crypto::{KeyPair, Hash},
    Transaction,
    time::IppanTime,
    merkle::compute_merkle_root,
};
use std::collections::BinaryHeap;
use std::sync::{Arc, RwLock};
use std::cmp::Ordering;

// Mempool entry for benchmarking
#[derive(Debug, Clone)]
struct MempoolEntry {
    transaction: Transaction,
    sort_key: (Hash, Hash),
}

impl PartialEq for MempoolEntry {
    fn eq(&self, other: &Self) -> bool {
        self.sort_key == other.sort_key
    }
}

impl Eq for MempoolEntry {}

impl PartialOrd for MempoolEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MempoolEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for max heap (highest priority first)
        other.sort_key.cmp(&self.sort_key)
    }
}

fn transaction_creation_benchmark(c: &mut Criterion) {
    c.bench_function("transaction_creation", |b| {
        b.iter(|| {
            let keypair = KeyPair::generate();
            let to_addr = [1u8; 32];
            let amount = 1000u64;
            let nonce = 1u64;
            let ippan_time_us = 1234567890u64;
            let hashtimer = [2u8; 32];
            let signature = [3u8; 64];
            
            let tx = Transaction::new(
                keypair.public_key,
                to_addr,
                amount,
                nonce,
                ippan_time_us,
                hashtimer,
                signature,
            );
            
            black_box(tx);
        });
    });
}

fn transaction_verification_benchmark(c: &mut Criterion) {
    let keypair = KeyPair::generate();
    let to_addr = [1u8; 32];
    let amount = 1000u64;
    let nonce = 1u64;
    let ippan_time_us = 1234567890u64;
    let hashtimer = [2u8; 32];
    
    let message = {
        let mut msg = Vec::new();
        msg.extend_from_slice(&1u8.to_le_bytes()); // version
        msg.extend_from_slice(&keypair.public_key);
        msg.extend_from_slice(&to_addr);
        msg.extend_from_slice(&amount.to_le_bytes());
        msg.extend_from_slice(&nonce.to_le_bytes());
        msg.extend_from_slice(&ippan_time_us.to_le_bytes());
        msg.extend_from_slice(&hashtimer);
        msg
    };
    
    let signature = keypair.sign(&message).unwrap();
    
    let tx = Transaction::new(
        keypair.public_key,
        to_addr,
        amount,
        nonce,
        ippan_time_us,
        hashtimer,
        signature,
    );
    
    c.bench_function("transaction_verification", |b| {
        b.iter(|| {
            let result = tx.verify();
            black_box(result);
        });
    });
}

fn ed25519_batch_verify_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519_batch_verify");
    
    for batch_size in [10, 50, 100, 500, 1000] {
        group.bench_function(&format!("batch_size_{}", batch_size), |b| {
            b.iter(|| {
                let keypairs: Vec<KeyPair> = (0..batch_size).map(|_| KeyPair::generate()).collect();
                let messages: Vec<&[u8]> = (0..batch_size).map(|i| format!("message {}", i).as_bytes()).collect();
                let signatures: Vec<_> = keypairs.iter()
                    .zip(messages.iter())
                    .map(|(kp, msg)| kp.sign(msg).unwrap())
                    .collect();
                
                let public_keys: Vec<_> = keypairs.iter()
                    .map(|kp| kp.public_key)
                    .collect();
                
                let results = KeyPair::batch_verify(&public_keys, &messages, &signatures);
                black_box(results);
            });
        });
    }
    
    group.finish();
}

fn mempool_enqueue_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("mempool_enqueue");
    
    for queue_size in [100, 1000, 10000] {
        group.bench_function(&format!("queue_size_{}", queue_size), |b| {
            b.iter(|| {
                let mut queue = BinaryHeap::new();
                
                for i in 0..queue_size {
                    let keypair = KeyPair::generate();
                    let to_addr = [1u8; 32];
                    let amount = 1000u64;
                    let nonce = i as u64;
                    let ippan_time_us = 1234567890u64;
                    let hashtimer = [2u8; 32];
                    let signature = [3u8; 64];
                    
                    let tx = Transaction::new(
                        keypair.public_key,
                        to_addr,
                        amount,
                        nonce,
                        ippan_time_us,
                        hashtimer,
                        signature,
                    );
                    
                    let sort_key = tx.get_sort_key().unwrap();
                    let entry = MempoolEntry { transaction: tx, sort_key };
                    queue.push(entry);
                }
                
                black_box(queue);
            });
        });
    }
    
    group.finish();
}

fn block_build_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_build");
    
    for tx_count in [100, 500, 1000, 5000] {
        group.bench_function(&format!("tx_count_{}", tx_count), |b| {
            b.iter(|| {
                let mut tx_ids = Vec::new();
                
                for _ in 0..tx_count {
                    let tx_id = [rand::random::<u8>(); 32];
                    tx_ids.push(tx_id);
                }
                
                let merkle_root = compute_merkle_root(&tx_ids);
                black_box(merkle_root);
            });
        });
    }
    
    group.finish();
}

fn merkle_root_computation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root");
    
    for tx_count in [1, 10, 100, 1000, 10000] {
        group.bench_function(&format!("tx_count_{}", tx_count), |b| {
            b.iter(|| {
                let tx_ids: Vec<Hash> = (0..tx_count).map(|i| {
                    let mut hash = [0u8; 32];
                    hash[0] = i as u8;
                    hash
                }).collect();
                
                let root = compute_merkle_root(&tx_ids);
                black_box(root);
            });
        });
    }
    
    group.finish();
}

fn hashtimer_generation_benchmark(c: &mut Criterion) {
    c.bench_function("hashtimer_generation", |b| {
        b.iter(|| {
            let ippan_time = 1234567890u64;
            let entropy = b"entropy";
            let tx_id = [1u8; 32];
            
            let hashtimer = ippan_common::crypto::generate_hash_timer(ippan_time, entropy, &tx_id);
            black_box(hashtimer);
        });
    });
}

fn address_derivation_benchmark(c: &mut Criterion) {
    c.bench_function("address_derivation", |b| {
        b.iter(|| {
            let keypair = KeyPair::generate();
            let address = ippan_common::crypto::derive_address(&keypair.public_key);
            black_box(address);
        });
    });
}

criterion_group!(
    benches,
    transaction_creation_benchmark,
    transaction_verification_benchmark,
    ed25519_batch_verify_benchmark,
    mempool_enqueue_benchmark,
    block_build_benchmark,
    merkle_root_computation_benchmark,
    hashtimer_generation_benchmark,
    address_derivation_benchmark,
);
criterion_main!(benches);
