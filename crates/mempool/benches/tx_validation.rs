use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ed25519_dalek::SigningKey;
use ippan_mempool::Mempool;
use ippan_types::{Amount, HashTimer, IppanTimeMicros, Transaction, TransactionVisibility};
use rand::{rngs::StdRng, Rng, SeedableRng};

const BATCH_SIZE: usize = 512;
const SIGNING_SEED: [u8; 32] = [42u8; 32];

fn tx_payload(from: &[u8; 32], to: &[u8; 32], amount: Amount, nonce: u64) -> Vec<u8> {
    let mut payload = Vec::with_capacity(32 * 2 + 16 + 8 + 1);
    payload.extend_from_slice(from);
    payload.extend_from_slice(to);
    payload.extend_from_slice(&amount.atomic().to_be_bytes());
    payload.extend_from_slice(&nonce.to_be_bytes());
    payload.push(0); // no handle operation
    payload
}

fn build_transaction(to: [u8; 32], nonce: u64) -> Transaction {
    let signing_key = SigningKey::from_bytes(&SIGNING_SEED);
    let from = signing_key.verifying_key().to_bytes();
    let amount = Amount::from_micro_ipn(10 + nonce as u64);
    let timestamp = IppanTimeMicros(1_700_000_000 + nonce);
    let payload = tx_payload(&from, &to, amount, nonce);
    let nonce_bytes = nonce.to_be_bytes();
    let hashtimer = HashTimer::derive(
        "transaction",
        timestamp,
        b"transaction",
        &payload,
        &nonce_bytes,
        &from,
    );

    let mut tx = Transaction {
        id: [0u8; 32],
        from,
        to,
        amount,
        nonce,
        hashtimer,
        timestamp,
        visibility: TransactionVisibility::Public,
        topics: Vec::new(),
        handle_op: None,
        confidential: None,
        zk_proof: None,
        signature: [0u8; 64],
    };

    let private = signing_key.to_bytes();
    tx.sign(&private).expect("sign deterministic transaction");
    tx
}

fn generate_transactions(count: usize) -> Vec<Transaction> {
    let mut rng = StdRng::seed_from_u64(2025);
    (0..count as u64)
        .map(|nonce| {
            let mut to = [0u8; 32];
            rng.fill(&mut to);
            build_transaction(to, nonce + 1)
        })
        .collect()
}

fn benchmark_tx_validation(c: &mut Criterion) {
    let txs = generate_transactions(BATCH_SIZE);

    let mut group = c.benchmark_group("mempool_tx_validation");
    group.throughput(Throughput::Elements(BATCH_SIZE as u64));
    group.bench_function("validate_batch_512", |b| {
        b.iter(|| {
            let pool = Mempool::new(BATCH_SIZE * 2);
            for tx in txs.iter() {
                pool.add_transaction(tx.clone()).expect("tx admitted");
            }
        });
    });
    group.finish();
}

criterion_group!(benches, benchmark_tx_validation);
criterion_main!(benches);
