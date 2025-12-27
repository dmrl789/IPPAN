use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use bincode::Options;
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use ippan_types::address::{decode_address, encode_address};
use ippan_types::{Amount, Transaction, TransactionWireV1};
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
    /// Batch mode: pre-sign raw transactions, frame them as binary, and POST to /tx/submit_batch.
    #[command(name = "batch", alias = "run-batch")]
    Batch(BatchArgs),
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

#[derive(Debug, Parser, Clone)]
struct BatchArgs {
    /// RPC base URL (e.g. http://127.0.0.1:8080). Scripts typically pass: http://127.0.0.1:8080 (run on api1)
    #[arg(long)]
    rpc: String,

    /// Target transactions per second (offered). Actual HTTP request rate depends on batching.
    #[arg(long)]
    tps: u64,

    /// Run duration (seconds)
    #[arg(long)]
    seconds: u64,

    /// Transactions per HTTP batch request.
    #[arg(long, default_value_t = 1024)]
    batch_size: u32,

    /// Concurrent worker tasks submitting batch requests.
    #[arg(long, default_value_t = 16)]
    concurrency: u32,

    /// Max in-flight prepared txs held client-side (backpressure to tx builder).
    /// Defaults to 2*concurrency.
    #[arg(long)]
    max_inflight: Option<u32>,

    /// Alias for --max-inflight (kept for older scripts).
    #[arg(long)]
    max_queue: Option<u32>,

    /// After the timed run ends, keep draining the internal queue up to this many seconds.
    #[arg(long, default_value_t = 10)]
    drain_seconds: u32,

    /// Path to a sender private key file containing hex (32 bytes / 64 hex chars).
    #[arg(long, alias = "sender-key")]
    from_key: Option<PathBuf>,

    /// Path to a JSON array of senders (compatible with `gen-senders` output).
    #[arg(long)]
    senders_file: Option<PathBuf>,

    /// Raw destination address (no handles). If omitted, a random address is generated.
    #[arg(long)]
    to: Option<String>,

    /// Starting nonce to use (per sender). Must be pre-reserved.
    #[arg(long)]
    nonce_start: u64,

    /// Number of nonces reserved (per sender). If omitted, defaults to `tps*seconds` (per sender).
    #[arg(long)]
    nonce_count: Option<u64>,

    /// If /health fails twice consecutively, stop early and write RPC_UNHEALTHY_STOP.txt (in cwd).
    #[arg(long, default_value_t = true)]
    stop_on_unhealthy: bool,
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
    Err(anyhow!(
        "missing signing key: provide --signing-key-hex or --signing-key-file"
    ))
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

fn bincode_tx_options() -> impl bincode::Options {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .reject_trailing_bytes()
}

fn build_batch_body(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    for frame in frames {
        out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        out.extend_from_slice(frame);
    }
    out
}

#[derive(Debug, Deserialize, Serialize)]
struct SubmitBatchResponse {
    accepted: usize,
    rejected: usize,
    http_429: usize,
    invalid: usize,
    elapsed_ms: u64,
}

#[derive(Default)]
struct BatchMetrics {
    ticks: AtomicUsize,
    enqueued: AtomicUsize,
    accepted: AtomicUsize,
    rejected: AtomicUsize,
    http_429: AtomicUsize,
    invalid: AtomicUsize,
    http_other: AtomicUsize,
    client_timeouts: AtomicUsize,
    client_connect: AtomicUsize,
    client_read: AtomicUsize,
    client_other: AtomicUsize,
    dropped_queue_full: AtomicUsize,
}

struct SenderRuntime {
    signing_key: SigningKey,
    from_pub: [u8; 32],
    to_pub: [u8; 32],
    nonce: u64,
}

fn load_senders_for_batch(args: &BatchArgs, to_pub: [u8; 32]) -> Result<Vec<SenderRuntime>> {
    match (&args.from_key, &args.senders_file) {
        (Some(_), Some(_)) => {
            return Err(anyhow!(
                "provide exactly one of --from-key/--sender-key or --senders-file"
            ))
        }
        (None, None) => {
            return Err(anyhow!(
                "missing sender: provide --sender-key or --senders-file"
            ))
        }
        _ => {}
    }

    if let Some(path) = &args.from_key {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read sender key file {}", path.display()))?;
        let signing_key = decode_signing_key_hex(&raw)?;
        let from_pub = signing_key.verifying_key().to_bytes();
        let nonce = args.nonce_start;
        return Ok(vec![SenderRuntime {
            signing_key,
            from_pub,
            to_pub,
            nonce,
        }]);
    }

    let path = args.senders_file.as_ref().expect("checked above");
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut parsed: Vec<Sender> =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if parsed.is_empty() {
        return Err(anyhow!("senders file is empty"));
    }

    let mut runtimes: Vec<SenderRuntime> = Vec::with_capacity(parsed.len());
    for sender in parsed.drain(..) {
        let signing_key = decode_signing_key_hex(&sender.signing_key_hex)?;
        let from_pub = signing_key.verifying_key().to_bytes();

        // Ensure the sender file is internally consistent (address matches signing key).
        let expected_pub = decode_address(&sender.address)
            .map_err(|e| anyhow!("invalid sender address {}: {e}", sender.address))?;
        if expected_pub != from_pub {
            return Err(anyhow!(
                "sender address does not match signing key (address={}, derived={})",
                sender.address,
                encode_address_from_pubkey(&from_pub)
            ));
        }

        let nonce = args.nonce_start.max(sender.nonce_start.max(1));
        runtimes.push(SenderRuntime {
            signing_key,
            from_pub,
            to_pub,
            nonce,
        });
    }

    Ok(runtimes)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Run(args) => run(args).await,
        Command::Batch(args) => run_batch(args).await,
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

    let signing_key =
        signing_key_from_args(args.signing_key_hex.clone(), args.signing_key_file.clone())?;
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
        resolve_nonce_start(&client, &args.rpc, &from)
            .await
            .unwrap_or(1)
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
                producer_metrics
                    .dropped_queue_full
                    .fetch_add(1, Ordering::Relaxed);
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
        w.abort();
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

async fn run_batch(args: BatchArgs) -> Result<()> {
    if args.tps == 0 {
        return Err(anyhow!("--tps must be > 0"));
    }
    if args.seconds == 0 {
        return Err(anyhow!("--seconds must be > 0"));
    }
    if args.batch_size == 0 {
        return Err(anyhow!("--batch-size must be > 0"));
    }
    if args.concurrency == 0 {
        return Err(anyhow!("--concurrency must be > 0"));
    }
    if args.nonce_start == 0 {
        return Err(anyhow!("--nonce-start must be > 0"));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build reqwest client")?;

    let endpoint = format!("{}/tx/submit_batch", args.rpc.trim_end_matches('/'));
    let health_url = format!("{}/health", args.rpc.trim_end_matches('/'));

    let to_pub = if let Some(raw) = &args.to {
        decode_address(raw).map_err(|e| anyhow!("invalid --to address: {e}"))?
    } else {
        SigningKey::generate(&mut OsRng).verifying_key().to_bytes()
    };

    let mut senders = load_senders_for_batch(&args, to_pub)?;
    let nonce_count = args
        .nonce_count
        .unwrap_or(args.tps.saturating_mul(args.seconds).max(1));
    let max_possible = (senders.len() as u64).saturating_mul(nonce_count);
    let total_target = args.tps.saturating_mul(args.seconds).max(1);
    if total_target > max_possible {
        return Err(anyhow!(
            "insufficient reserved nonces: need total={} but have senders={} * nonce_count={} = {}",
            total_target,
            senders.len(),
            nonce_count,
            max_possible
        ));
    }

    // Stop on unhealthy (two consecutive failures).
    let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    if args.stop_on_unhealthy {
        let client = client.clone();
        let url = health_url.clone();
        let stop = stop_flag.clone();
        tokio::spawn(async move {
            let mut fails = 0u32;
            let mut interval = tokio::time::interval(Duration::from_millis(1_000));
            loop {
                interval.tick().await;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let ok = client
                    .get(&url)
                    .timeout(Duration::from_secs(2))
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);
                if ok {
                    fails = 0;
                    continue;
                }
                fails += 1;
                if fails >= 2 {
                    stop.store(true, Ordering::Relaxed);
                    let _ = fs::write(
                        "RPC_UNHEALTHY_STOP.txt",
                        format!("RPC unhealthy (2 consecutive /health failures): {url}\n"),
                    );
                    break;
                }
            }
        });
    }

    let metrics = Arc::new(BatchMetrics::default());
    let max_inflight = args
        .max_inflight
        .or(args.max_queue)
        .unwrap_or(args.concurrency.saturating_mul(2))
        .max(1);
    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(max_inflight as usize);
    let rx = Arc::new(tokio::sync::Mutex::new(rx));

    let tps = args.tps;
    let ticks_every = Duration::from_nanos(1_000_000_000u64 / tps.max(1));
    let duration = Duration::from_secs(args.seconds);
    let start = Instant::now();

    // Producer: tick at target TPS, pre-build + pre-sign tx bytes into queue.
    let producer_metrics = Arc::clone(&metrics);
    let stop = stop_flag.clone();
    let producer = tokio::spawn(async move {
        let mut interval = tokio::time::interval(ticks_every);
        let mut rr = 0usize;
        while start.elapsed() < duration && !stop.load(Ordering::Relaxed) {
            interval.tick().await;
            producer_metrics.ticks.fetch_add(1, Ordering::Relaxed);

            let idx = rr % senders.len();
            rr = rr.wrapping_add(1);
            let sender = &mut senders[idx];

            let amount = Amount::from_atomic(1_000u128);
            let mut tx_obj = Transaction::new(sender.from_pub, sender.to_pub, amount, sender.nonce);
            let _ = tx_obj.sign(&sender.signing_key.to_bytes());
            sender.nonce = sender.nonce.saturating_add(1);

            // Convert to WireV1 for bincode-stable serialization
            let wire = TransactionWireV1::from(&tx_obj);
            let tx_bytes = match bincode_tx_options().serialize(&wire) {
                Ok(b) => b,
                Err(_) => continue,
            };

            if tx.try_send(tx_bytes).is_ok() {
                producer_metrics.enqueued.fetch_add(1, Ordering::Relaxed);
            } else {
                producer_metrics
                    .dropped_queue_full
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    // Workers: submit binary batches.
    let mut workers = Vec::with_capacity(args.concurrency as usize);
    for _ in 0..args.concurrency {
        let client = client.clone();
        let endpoint = endpoint.clone();
        let metrics = Arc::clone(&metrics);
        let rx = Arc::clone(&rx);
        let batch_size = args.batch_size as usize;
        let stop = stop_flag.clone();
        workers.push(tokio::spawn(async move {
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let mut frames: Vec<Vec<u8>> = Vec::with_capacity(batch_size.max(1));
                {
                    let mut guard = rx.lock().await;
                    let first = guard.recv().await;
                    let Some(first) = first else { break };
                    frames.push(first);
                    while frames.len() < batch_size {
                        match guard.try_recv() {
                            Ok(next) => frames.push(next),
                            Err(_) => break,
                        }
                    }
                }

                let body = build_batch_body(&frames);
                let batch_len = frames.len();
                let resp = client
                    .post(&endpoint)
                    .header("Content-Type", "application/octet-stream")
                    .body(body)
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.status().is_success() => match r.json::<SubmitBatchResponse>().await
                    {
                        Ok(parsed) => {
                            metrics
                                .accepted
                                .fetch_add(parsed.accepted, Ordering::Relaxed);
                            metrics
                                .rejected
                                .fetch_add(parsed.rejected, Ordering::Relaxed);
                            metrics
                                .http_429
                                .fetch_add(parsed.http_429, Ordering::Relaxed);
                            metrics.invalid.fetch_add(parsed.invalid, Ordering::Relaxed);
                        }
                        Err(_) => {
                            metrics.client_read.fetch_add(1, Ordering::Relaxed);
                            metrics.http_other.fetch_add(1, Ordering::Relaxed);
                        }
                    },
                    Ok(r) if r.status() == StatusCode::TOO_MANY_REQUESTS => {
                        match r.json::<SubmitBatchResponse>().await {
                            Ok(parsed) => {
                                metrics
                                    .accepted
                                    .fetch_add(parsed.accepted, Ordering::Relaxed);
                                metrics
                                    .rejected
                                    .fetch_add(parsed.rejected, Ordering::Relaxed);
                                metrics.invalid.fetch_add(parsed.invalid, Ordering::Relaxed);
                                let mut over = parsed.http_429;
                                if over == 0 {
                                    over = batch_len;
                                }
                                metrics.http_429.fetch_add(over, Ordering::Relaxed);
                            }
                            Err(_) => {
                                // Treat malformed/empty 429 bodies as overload without counting a client error.
                                metrics.http_429.fetch_add(batch_len, Ordering::Relaxed);
                            }
                        }
                    }
                    Ok(_) => {
                        metrics.http_other.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(err) => {
                        if err.is_timeout() {
                            metrics.client_timeouts.fetch_add(1, Ordering::Relaxed);
                        } else if err.is_connect() {
                            metrics.client_connect.fetch_add(1, Ordering::Relaxed);
                        } else {
                            metrics.client_other.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        }));
    }

    let _ = producer.await;

    // Drain window: allow workers to finish queued work (bounded by drain_seconds).
    let drain_deadline = Instant::now() + Duration::from_secs(args.drain_seconds as u64);
    let mut join_handles = workers;
    loop {
        let mut remaining = Vec::with_capacity(join_handles.len());
        for h in join_handles {
            if h.is_finished() {
                let _ = h.await;
            } else {
                remaining.push(h);
            }
        }
        join_handles = remaining;

        if join_handles.is_empty() || Instant::now() >= drain_deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    for h in join_handles {
        h.abort();
    }

    let elapsed = (args.seconds as f64).max(0.001);
    let accepted = metrics.accepted.load(Ordering::Relaxed);
    let accepted_tps = accepted as f64 / elapsed;
    let rejected = metrics.rejected.load(Ordering::Relaxed);
    let http_429 = metrics.http_429.load(Ordering::Relaxed);
    let invalid = metrics.invalid.load(Ordering::Relaxed);
    let dropped_queue_full = metrics.dropped_queue_full.load(Ordering::Relaxed);

    let client_timeouts = metrics.client_timeouts.load(Ordering::Relaxed);
    let client_connect = metrics.client_connect.load(Ordering::Relaxed);
    let client_read = metrics.client_read.load(Ordering::Relaxed);
    let client_other = metrics.client_other.load(Ordering::Relaxed);
    let client_errors = client_timeouts + client_connect + client_read + client_other;

    println!("RPC: {}", args.rpc);
    println!("Endpoint: {}", endpoint);
    println!("offered_tps: {}", args.tps);
    println!("duration_s: {}", args.seconds);
    println!("batch_size: {}", args.batch_size);
    println!("concurrency: {}", args.concurrency);
    println!("accepted: {}", accepted);
    println!("rejected: {}", rejected);
    println!("http_429: {}", http_429);
    println!("invalid: {}", invalid);
    println!("client_timeouts: {}", client_timeouts);
    println!("client_connect: {}", client_connect);
    println!("client_read: {}", client_read);
    println!("client_other: {}", client_other);
    println!("dropped_queue_full: {}", dropped_queue_full);
    println!("accepted_tps: {:.2}", accepted_tps);

    println!(
        "SUMMARY offered_tps={:.2} accepted_tps={:.2} http_429={} invalid={} client_errors={} dropped_queue_full={}",
        args.tps as f64,
        accepted_tps,
        http_429,
        invalid,
        client_errors,
        dropped_queue_full
    );

    Ok(())
}
