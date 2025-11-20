use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use ed25519_dalek::SigningKey;
use ippan_types::address::encode_address;
use rand::rngs::OsRng;
use reqwest::blocking::Client;
use serde::Serialize;

#[derive(Debug, Parser, Clone)]
#[command(author, version, about = "IPPAN RPC load generator", long_about = None)]
struct Args {
    /// RPC base URL (e.g. http://127.0.0.1:3000)
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    rpc: String,

    /// Number of payment transactions to submit
    #[arg(long, default_value_t = 1_000)]
    tx_count: u64,

    /// Number of concurrent workers issuing requests
    #[arg(long, default_value_t = 8)]
    concurrency: usize,

    /// Optional signing key (hex). Randomly generated if omitted.
    #[arg(long)]
    signing_key: Option<String>,

    /// Optional destination address; generated if omitted.
    #[arg(long)]
    destination: Option<String>,

    /// Payment amount in atomic units
    #[arg(long, default_value_t = 1_000u128)]
    amount: u128,

    /// Optional fee limit (atomic units)
    #[arg(long)]
    fee_limit: Option<u128>,

    /// Starting nonce to use for generated payments
    #[arg(long, default_value_t = 1)]
    nonce_start: u64,

    /// Optional memo/topic attached to each payment
    #[arg(long)]
    memo: Option<String>,
}

#[derive(Clone, Serialize)]
struct PaymentRequest {
    from: String,
    to: String,
    amount: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
    signing_key: String,
}

fn parse_signing_key(raw: Option<String>) -> Result<SigningKey> {
    if let Some(raw) = raw {
        let normalized = raw.trim().trim_start_matches("0x");
        let bytes =
            hex::decode(normalized).map_err(|err| anyhow!("invalid signing key hex: {err}"))?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("signing key must be 32 bytes (64 hex chars)"))?;
        Ok(SigningKey::from_bytes(&key_bytes))
    } else {
        Ok(SigningKey::generate(&mut OsRng))
    }
}

fn derive_destination(destination: Option<String>) -> (String, SigningKey) {
    if let Some(address) = destination {
        return (address, SigningKey::generate(&mut OsRng));
    }

    let to_key = SigningKey::generate(&mut OsRng);
    let to_address = encode_address(&to_key.verifying_key().to_bytes());
    (to_address, to_key)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let rpc_endpoint = format!("{}/tx/payment", args.rpc.trim_end_matches('/'));

    let signing_key = parse_signing_key(args.signing_key.clone())?;
    let from_address = encode_address(&signing_key.verifying_key().to_bytes());
    let signing_key_hex = hex::encode(signing_key.to_bytes());
    let (to_address, _to_key) = derive_destination(args.destination.clone());

    let total = args.tx_count as usize;
    if total == 0 {
        return Err(anyhow!("tx-count must be greater than zero"));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")?;

    let counter = AtomicUsize::new(0);
    let success = AtomicUsize::new(0);
    let failures = AtomicUsize::new(0);
    let latencies = Arc::new(Mutex::new(Vec::with_capacity(total)));

    let start = Instant::now();
    thread::scope(|scope| {
        for _ in 0..args.concurrency.max(1) {
            let client = client.clone();
            let from = from_address.clone();
            let to = to_address.clone();
            let memo = args.memo.clone();
            let latencies = Arc::clone(&latencies);
            let fee = args.fee_limit;
            let signing_key_hex = signing_key_hex.clone();
            scope.spawn(move || loop {
                let idx = counter.fetch_add(1, Ordering::Relaxed);
                if idx >= total {
                    break;
                }

                let nonce = args.nonce_start.saturating_add(idx as u64);
                let payload = PaymentRequest {
                    from: from.clone(),
                    to: to.clone(),
                    amount: args.amount,
                    fee,
                    nonce: Some(nonce),
                    memo: memo.clone(),
                    signing_key: signing_key_hex.clone(),
                };

                let sent_at = Instant::now();
                let result = client.post(&rpc_endpoint).json(&payload).send();
                let elapsed = sent_at.elapsed();

                match result {
                    Ok(response) if response.status().is_success() => {
                        success.fetch_add(1, Ordering::Relaxed);
                        if let Ok(mut guard) = latencies.lock() {
                            guard.push(elapsed);
                        }
                    }
                    Ok(response) => {
                        failures.fetch_add(1, Ordering::Relaxed);
                        eprintln!("Request {} failed with status {}", idx, response.status());
                    }
                    Err(err) => {
                        failures.fetch_add(1, Ordering::Relaxed);
                        eprintln!("Request {} errored: {}", idx, err);
                    }
                }
            });
        }
    });
    let duration = start.elapsed();

    let completed = success.load(Ordering::Relaxed);
    let failed = failures.load(Ordering::Relaxed);
    let tps = completed as f64 / duration.as_secs_f64();

    let latencies_ms: Vec<f64> = latencies
        .lock()
        .unwrap()
        .iter()
        .map(|d| d.as_secs_f64() * 1_000.0)
        .collect();
    let mean_latency = if latencies_ms.is_empty() {
        0.0
    } else {
        latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64
    };

    let mut sorted_latencies = latencies_ms.clone();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = if sorted_latencies.is_empty() {
        0.0
    } else {
        let idx = ((sorted_latencies.len() as f64) * 0.95).ceil() as usize - 1;
        sorted_latencies[idx.min(sorted_latencies.len() - 1)]
    };

    println!("RPC endpoint: {}", rpc_endpoint);
    println!("From: {}", from_address);
    println!("To: {}", to_address);
    println!("Transactions requested: {}", total);
    println!("Completed: {} | Failed: {}", completed, failed);
    println!("Elapsed: {:.2?} | TPS: {:.2}", duration, tps);
    println!(
        "Mean latency: {:.2} ms | p95 latency: {:.2} ms",
        mean_latency, p95
    );

    Ok(())
}
