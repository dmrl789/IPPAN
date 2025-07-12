#![feature(test)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan::{
    wallet::{WalletManager, WalletConfig},
    utils::crypto,
};
use std::time::Instant;

/// Benchmark wallet creation and initialization
fn benchmark_wallet_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Wallet Operations");
    
    group.bench_function("create_wallet_manager", |b| {
        b.iter(|| {
            let config = WalletConfig::default();
            black_box(WalletManager::new(config).unwrap())
        });
    });
    
    group.bench_function("wallet_start_stop", |b| {
        b.iter(|| {
            let config = WalletConfig::default();
            let mut wallet = WalletManager::new(config).unwrap();
            wallet.start().unwrap();
            wallet.stop().unwrap();
        });
    });
    
    group.finish();
}

/// Benchmark key generation and management
fn benchmark_key_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Key Operations");
    
    group.bench_function("generate_keypair", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        b.iter(|| {
            black_box(wallet.keys.write().unwrap().generate_keypair().unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("generate_node_id", |b| {
        b.iter(|| {
            black_box(crypto::generate_node_id())
        });
    });
    
    group.bench_function("hash_data", |b| {
        let test_data = b"Test data for hashing benchmark.";
        
        b.iter(|| {
            black_box(crypto::hash(test_data))
        });
    });
    
    group.bench_function("sign_message", |b| {
        let keypair = crypto::generate_keypair();
        let message = b"Test message for signing benchmark.";
        
        b.iter(|| {
            black_box(crypto::sign(message, &keypair.private_key))
        });
    });
    
    group.bench_function("verify_signature", |b| {
        let keypair = crypto::generate_keypair();
        let message = b"Test message for verification benchmark.";
        let signature = crypto::sign(message, &keypair.private_key);
        
        b.iter(|| {
            black_box(crypto::verify(message, &signature, &keypair.public_key))
        });
    });
    
    group.finish();
}

/// Benchmark payment processing
fn benchmark_payment_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Payment Operations");
    
    group.bench_function("process_payment", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        let keypair = wallet.keys.write().unwrap().generate_keypair().unwrap();
        
        b.iter(|| {
            black_box(wallet.payments.write().unwrap().process_payment(
                &keypair.public_key,
                1000,
                [1u8; 32],
            ).unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("process_multiple_payments", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        let keypair = wallet.keys.write().unwrap().generate_keypair().unwrap();
        
        b.iter(|| {
            for i in 0..10 {
                black_box(wallet.payments.write().unwrap().process_payment(
                    &keypair.public_key,
                    1000 + i as u64,
                    [i as u8; 32],
                ).unwrap());
            }
        });
        
        wallet.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark M2M payment operations
fn benchmark_m2m_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("M2M Payment Operations");
    
    group.bench_function("create_payment_channel", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        b.iter(|| {
            black_box(wallet.create_payment_channel(
                "alice".to_string(),
                "bob".to_string(),
                10000,
                24,
            ).unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("process_micro_payment", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        let channel = wallet.create_payment_channel(
            "sender".to_string(),
            "recipient".to_string(),
            10000,
            24,
        ).unwrap();
        
        b.iter(|| {
            black_box(wallet.process_micro_payment(
                &channel.channel_id,
                100,
                crate::wallet::m2m_payments::MicroTransactionType::DataTransfer { bytes_transferred: 1024 },
            ).unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("process_multiple_micro_payments", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        let channel = wallet.create_payment_channel(
            "sender".to_string(),
            "recipient".to_string(),
            10000,
            24,
        ).unwrap();
        
        b.iter(|| {
            for i in 0..10 {
                black_box(wallet.process_micro_payment(
                    &channel.channel_id,
                    100 + i as u64,
                    crate::wallet::m2m_payments::MicroTransactionType::SensorData {
                        sensor_type: "temperature".to_string(),
                        data_points: i as u32,
                    },
                ).unwrap());
            }
        });
        
        wallet.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark wallet statistics and queries
fn benchmark_wallet_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("Wallet Statistics");
    
    group.bench_function("get_balance", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        b.iter(|| {
            black_box(wallet.get_balance().unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("get_m2m_statistics", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        b.iter(|| {
            black_box(wallet.get_m2m_statistics().unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("get_total_m2m_fees", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        b.iter(|| {
            black_box(wallet.get_total_m2m_fees().unwrap())
        });
        
        wallet.stop().unwrap();
    });
    
    group.finish();
}

/// Benchmark cryptographic operations
fn benchmark_crypto_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Cryptographic Operations");
    
    group.bench_function("sha256_hash", |b| {
        let test_data = b"Test data for SHA256 benchmarking.";
        
        b.iter(|| {
            black_box(crypto::hash(test_data))
        });
    });
    
    group.bench_function("sha256_hash_large_data", |b| {
        let test_data = vec![0u8; 1024 * 1024]; // 1MB
        
        b.iter(|| {
            black_box(crypto::hash(&test_data))
        });
    });
    
    group.bench_function("ed25519_key_generation", |b| {
        b.iter(|| {
            black_box(crypto::generate_keypair())
        });
    });
    
    group.bench_function("ed25519_sign_verify", |b| {
        let keypair = crypto::generate_keypair();
        let message = b"Test message for Ed25519 benchmark.";
        
        b.iter(|| {
            let signature = crypto::sign(message, &keypair.private_key);
            black_box(crypto::verify(message, &signature, &keypair.public_key))
        });
    });
    
    group.bench_function("batch_sign_verify", |b| {
        let keypair = crypto::generate_keypair();
        let messages: Vec<&[u8]> = (0..10).map(|i| {
            format!("Test message {}", i).as_bytes()
        }).collect();
        
        b.iter(|| {
            for message in &messages {
                let signature = crypto::sign(message, &keypair.private_key);
                black_box(crypto::verify(message, &signature, &keypair.public_key));
            }
        });
    });
    
    group.finish();
}

/// Benchmark wallet throughput
fn benchmark_wallet_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Wallet Throughput");
    
    group.bench_function("concurrent_payments", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        let keypair = wallet.keys.write().unwrap().generate_keypair().unwrap();
        
        b.iter(|| {
            // Simulate concurrent payment processing
            for i in 0..100 {
                black_box(wallet.payments.write().unwrap().process_payment(
                    &keypair.public_key,
                    1000 + i as u64,
                    [i as u8; 32],
                ).unwrap());
            }
        });
        
        wallet.stop().unwrap();
    });
    
    group.bench_function("concurrent_m2m_payments", |b| {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).unwrap();
        wallet.start().unwrap();
        
        let channel = wallet.create_payment_channel(
            "sender".to_string(),
            "recipient".to_string(),
            100000,
            24,
        ).unwrap();
        
        b.iter(|| {
            // Simulate concurrent M2M payments
            for i in 0..100 {
                black_box(wallet.process_micro_payment(
                    &channel.channel_id,
                    10 + i as u64,
                    crate::wallet::m2m_payments::MicroTransactionType::SensorData {
                        sensor_type: "temperature".to_string(),
                        data_points: i as u32,
                    },
                ).unwrap());
            }
        });
        
        wallet.stop().unwrap();
    });
    
    group.finish();
}

criterion_group!(
    wallet_benches,
    benchmark_wallet_operations,
    benchmark_key_operations,
    benchmark_payment_operations,
    benchmark_m2m_operations,
    benchmark_wallet_statistics,
    benchmark_crypto_operations,
    benchmark_wallet_throughput
);
criterion_main!(wallet_benches); 