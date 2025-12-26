use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use ippan_types::address::encode_address;
use rand::rngs::OsRng;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Single-sender load: rate-limited payment submission with metrics.
    Run(RunArgs),
    /// Generate a JSON file with N random senders (signing keys + derived addresses).
    GenSenders(GenSendersArgs),
}

#[derive(Debug, Parser, Clone)]
struct RunArgs {
    /// RPC base URL (e.g. http://127.0.0.1:8080). Scripts typically pass tunnel: http://127.0.0.1:18080
    #[arg(long)]
    rpc: String,

    /// Target transactions per second
    #[arg(long)]
    tps: u32,

    /// Run duration (seconds)
    #[arg(long)]
    seconds: u64,

    /// Concurrent worker tasks submitting requests
    #[arg(long, default_value_t = 16)]
    concurrency: usize,

    /// Amount in atomic units
    #[arg(long, default_value_t = 1_000u128)]
    amount: u128,

    /// Optional fee in atomic units
    #[arg(long)]
    fee: Option<u128>,

    /// Signing key as hex (32 bytes / 64 hex chars). If omitted, use --signing-key-file.
    #[arg(long)]
    signing_key_hex: Option<String>,

    /// Path to signing key file containing hex.
    #[arg(long)]
    signing_key_file: Option<PathBuf>,

    /// Optional explicit "from" address/handle. If omitted, derived from signing key.
    #[arg(long)]
    from: Option<String>,

    /// Optional destination address/handle. If omitted, random address is generated.
    #[arg(long)]
    to: Option<String>,

    /// Starting nonce. If omitted, query /account/:address and start at (nonce+1) when possible.
    #[arg(long)]
    nonce_start: Option<u64>,

    /// Max in-flight queue depth (producer will drop if full).
    #[arg(long, default_value_t = 10_000)]
    max_queue: usize,
}

#[derive(Debug, Parser)]
struct GenSendersArgs {
    /// Number of senders to generate
    #[arg(long)]
    count: usize,
    /// Output JSON path
    #[arg(long, default_value = "out/senders/senders.json")]
    out: PathBuf,
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
    signing_key: String,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    #[serde(default)]
    nonce: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Sender {
    signing_key_hex: String,
    address: String,
    #[serde(default)]
    nonce_start: u64,
}

fn decode_signing_key_hex(raw: &str) -> Result<SigningKey> {
    let normalized = raw.trim().trim_start_matches("0x");
    let bytes = hex::decode(normalized).context("decode signing key hex")?;
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("signing key must be 32 bytes (64 hex chars)"))?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

fn signing_key_from_args(hex_opt: Option<String>, file_opt: Option<PathBuf>) -> Result<SigningKey> {
    if let Some(raw) = hex_opt {
        return decode_signing_key_hex(&raw);
    }
    if let Some(path) = file_opt {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("read signing key file {}", path.display()))?;
        return decode_signing_key_hex(&raw);
    }
    Err(anyhow!("missing signing key: provide --signing-key-hex or --signing-key-file"))
}

fn encode_address_from_pubkey(pubkey32: &[u8; 32]) -> String {
    // Must match node-side parsing expectations (Base58Check-style).
    encode_address(pubkey32)
}

async fn resolve_nonce_start(client: &reqwest::Client, rpc: &str, from: &str) -> Option<u64> {
    let url = format!("{}/account/{}", rpc.trim_end_matches('/'), from);
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: AccountResponse = resp.json().await.ok()?;
    Some(parsed.nonce.saturating_add(1))
}

#[derive(Default)]
struct Metrics {
    submitted: AtomicUsize,
    accepted: AtomicUsize,
    http_429: AtomicUsize,
    http_other: AtomicUsize,
    client_errors: AtomicUsize,
    dropped_queue_full: AtomicUsize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Run(args) => run(args).await,
        Command::GenSenders(args) => gen_senders(args),
    }
}

fn gen_senders(args: GenSendersArgs) -> Result<()> {
    let mut rng = OsRng;
    let mut out = Vec::with_capacity(args.count);
    for _ in 0..args.count {
        let sk = SigningKey::generate(&mut rng);
        let pubkey = sk.verifying_key().to_bytes();
        let address = encode_address_from_pubkey(&pubkey);
        out.push(Sender {
            signing_key_hex: hex::encode(sk.to_bytes()),
            address,
            nonce_start: 1,
        });
    }
    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(&args.out, serde_json::to_vec_pretty(&out)?)
        .with_context(|| format!("write {}", args.out.display()))?;
    println!("Wrote {} senders to {}", out.len(), args.out.display());
    Ok(())
}

async fn run(args: RunArgs) -> Result<()> {
    if args.tps == 0 {
        return Err(anyhow!("--tps must be > 0"));
    }
    if args.seconds == 0 {
        return Err(anyhow!("--seconds must be > 0"));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build reqwest client")?;

    let signing_key = signing_key_from_args(args.signing_key_hex.clone(), args.signing_key_file.clone())?;
    let pubkey = signing_key.verifying_key().to_bytes();
    let derived_from = encode_address_from_pubkey(&pubkey);
    let from = args.from.clone().unwrap_or(derived_from.clone());
    let signing_key_hex = hex::encode(signing_key.to_bytes());

    let to = if let Some(to) = args.to.clone() {
        to
    } else {
        let to_key = SigningKey::generate(&mut OsRng);
        encode_address_from_pubkey(&to_key.verifying_key().to_bytes())
    };

    let nonce_start = if let Some(n) = args.nonce_start {
        n
    } else {
        resolve_nonce_start(&client, &args.rpc, &from).await.unwrap_or(1)
    };
    let nonce = Arc::new(AtomicU64::new(nonce_start));

    let endpoint = format!("{}/tx/payment", args.rpc.trim_end_matches('/'));
    let metrics = Arc::new(Metrics::default());

    let (tx, rx) = tokio::sync::mpsc::channel::<u64>(args.max_queue.max(1));
    let rx = Arc::new(tokio::sync::Mutex::new(rx));

    // Producer: tick at target TPS, enqueue nonces.
    let producer_metrics = Arc::clone(&metrics);
    let producer_nonce = Arc::clone(&nonce);
    let tps = args.tps as u64;
    let ticks_every = Duration::from_nanos(1_000_000_000u64 / tps.max(1));
    let duration = Duration::from_secs(args.seconds);
    let start = Instant::now();
    let producer = tokio::spawn(async move {
        let mut interval = tokio::time::interval(ticks_every);
        while start.elapsed() < duration {
            interval.tick().await;
            let n = producer_nonce.fetch_add(1, Ordering::Relaxed);
            producer_metrics.submitted.fetch_add(1, Ordering::Relaxed);
            if tx.try_send(n).is_err() {
                producer_metrics.dropped_queue_full.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    // Workers: submit payments.
    let mut workers = Vec::with_capacity(args.concurrency.max(1));
    for _ in 0..args.concurrency.max(1) {
        let client = client.clone();
        let endpoint = endpoint.clone();
        let from = from.clone();
        let to = to.clone();
        let signing_key_hex = signing_key_hex.clone();
        let fee = args.fee;
        let amount = args.amount;
        let metrics = Arc::clone(&metrics);
        let rx = Arc::clone(&rx);
        workers.push(tokio::spawn(async move {
            loop {
                let nonce = {
                    let mut guard = rx.lock().await;
                    guard.recv().await
                };
                let Some(nonce) = nonce else { break };
                let payload = PaymentRequest {
                    from: from.clone(),
                    to: to.clone(),
                    amount,
                    fee,
                    nonce: Some(nonce),
                    signing_key: signing_key_hex.clone(),
                };
                let resp = client.post(&endpoint).json(&payload).send().await;
                match resp {
                    Ok(r) if r.status().is_success() => {
                        metrics.accepted.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(r) if r.status() == StatusCode::TOO_MANY_REQUESTS => {
                        metrics.http_429.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(_r) => {
                        metrics.http_other.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        metrics.client_errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }
    // Dropping the last Sender closes the channel once the producer exits.

    let _ = producer.await;
    // Give workers a short drain window.
    tokio::time::sleep(Duration::from_millis(250)).await;

    for w in workers {
        let _ = w.abort();
    }

    let elapsed = start.elapsed().as_secs_f64().max(0.001);
    let accepted = metrics.accepted.load(Ordering::Relaxed);
    let accepted_tps = accepted as f64 / elapsed;
    let submitted = metrics.submitted.load(Ordering::Relaxed);
    let http_429 = metrics.http_429.load(Ordering::Relaxed);
    let http_other = metrics.http_other.load(Ordering::Relaxed);
    let client_errors = metrics.client_errors.load(Ordering::Relaxed);
    let dropped = metrics.dropped_queue_full.load(Ordering::Relaxed);

    println!("RPC: {}", args.rpc);
    println!("Endpoint: {}", endpoint);
    println!("From: {}", from);
    println!("To: {}", to);
    println!("nonce_start: {}", nonce_start);
    println!("target_tps: {}", args.tps);
    println!("duration_s: {}", args.seconds);
    println!("concurrency: {}", args.concurrency);
    println!("submitted: {}", submitted);
    println!("accepted: {}", accepted);
    println!("http_429: {}", http_429);
    println!("http_other: {}", http_other);
    println!("client_errors: {}", client_errors);
    println!("dropped_queue_full: {}", dropped);
    println!("accepted_tps: {:.2}", accepted_tps);
    // One-line summary for scripts/tee parsing.
    println!(
        "SUMMARY accepted_tps={:.2} http_429={} http_other={} client_errors={} dropped_queue_full={}",
        accepted_tps, http_429, http_other, client_errors, dropped
    );

    Ok(())
}


