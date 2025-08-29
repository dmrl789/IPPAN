use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ippan::{
    crypto::KeyPair,
    transaction::Transaction,
    time::IppanTime,
    mempool::Mempool,
    block::{Block, BlockBuilder},
};
use std::sync::Arc;

fn transaction_creation_benchmark(c: &mut Criterion) {
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    c.bench_function("transaction_creation", |b| {
        b.iter(|| {
            let _tx = Transaction::new(
                black_box(&keypair),
                black_box(recipient.public_key),
                black_box(1000),
                black_box(1),
                black_box(ippan_time.clone()),
            ).unwrap();
        });
    });
}

fn transaction_verification_benchmark(c: &mut Criterion) {
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    let tx = Transaction::new(
        &keypair,
        recipient.public_key,
        1000,
        1,
        ippan_time,
    ).unwrap();
    
    c.bench_function("transaction_verification", |b| {
        b.iter(|| {
            black_box(tx.verify()).unwrap();
        });
    });
}

fn mempool_operations_benchmark(c: &mut Criterion) {
    let mempool = Mempool::new(4);
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    c.bench_function("mempool_add_transaction", |b| {
        b.iter(|| {
            let tx = Transaction::new(
                &keypair,
                recipient.public_key,
                1000,
                1,
                ippan_time.clone(),
            ).unwrap();
            
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                black_box(mempool.add_transaction(tx).await).unwrap();
            });
        });
    });
}

fn block_creation_benchmark(c: &mut Criterion) {
    let builder = BlockBuilder::new();
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    // Create some transactions
    let transactions: Vec<Transaction> = (0..100)
        .map(|i| {
            Transaction::new(
                &keypair,
                recipient.public_key,
                1000,
                i + 1,
                ippan_time.clone(),
            ).unwrap()
        })
        .collect();
    
    c.bench_function("block_creation", |b| {
        b.iter(|| {
            let _block = builder.build_block(
                black_box(vec![]),
                black_box(1),
                black_box(1234567890),
                black_box([1u8; 32]),
                black_box(transactions.clone()),
            ).unwrap();
        });
    });
}

fn batch_transaction_processing_benchmark(c: &mut Criterion) {
    let mempool = Mempool::new(4);
    let keypair = KeyPair::generate();
    let recipient = KeyPair::generate();
    let ippan_time = Arc::new(IppanTime::new());
    
    c.bench_function("batch_transaction_processing", |b| {
        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                // Add 1000 transactions
                for i in 0..1000 {
                    let tx = Transaction::new(
                        &keypair,
                        recipient.public_key,
                        1000,
                        i + 1,
                        ippan_time.clone(),
                    ).unwrap();
                    
                    mempool.add_transaction(tx).await.unwrap();
                }
                
                // Retrieve transactions for block
                let transactions = mempool.get_transactions_for_block(100).await;
                black_box(transactions.len());
            });
        });
    });
}

criterion_group!(
    benches,
    transaction_creation_benchmark,
    transaction_verification_benchmark,
    mempool_operations_benchmark,
    block_creation_benchmark,
    batch_transaction_processing_benchmark,
);
criterion_main!(benches);
