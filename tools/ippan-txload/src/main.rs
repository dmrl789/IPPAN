use anyhow::{anyhow, Context, Result};
use clap::Parser;
use hdrhistogram::Histogram;
use ippan_wallet::keyfile::KeyFile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Semaphore};

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum NonceMode {
    /// Send nonce field as provided (nonce_start + i)
    Provide,
    /// Omit nonce field entirely; node derives nonce
    Omit,
}

#[derive(Debug, Parser, Clone)]
#[command(author, version, about = "IPPAN true tx load test generator (POST /tx/payment)")]
struct Args {
    /// RPC base URL (e.g. http://188.245.97.41:8080)
    #[arg(long)]
    rpc: String,

    /// Target transactions per second
    #[arg(long, default_value_t = 2000)]
    tps: u64,

    /// Duration of the load run in seconds
    #[arg(long, default_value_t = 600)]
    duration: u64,

    /// Number of in-flight requests allowed
    #[arg(long, default_value_t = 200)]
    concurrency: usize,

    /// Path to sender wallet key file (ippan-wallet JSON keyfile)
    #[arg(long, value_name = "PATH")]
    from_key: PathBuf,

    /// Optional password to unlock the keyfile (can also be provided via IPPAN_KEY_PASSWORD env)
    #[arg(long, value_name = "PASSWORD")]
    from_key_password: Option<String>,

    /// Recipient identifier (Base58Check, hex, or @handle)
    #[arg(long)]
    to: String,

    /// Amount in atomic units
    #[arg(long, default_value_t = 1)]
    amount: u128,

    /// Optional fee limit in atomic units
    #[arg(long)]
    fee: Option<u128>,

    /// Optional memo/topic (<=256 bytes recommended)
    #[arg(long, default_value = "loadtest")]
    memo: String,

    /// Optional explicit starting nonce (otherwise fetched from RPC)
    #[arg(long)]
    nonce_start: Option<u64>,

    /// Nonce mode: provide (default) or omit (node derives)
    #[arg(long, value_enum, default_value_t = NonceMode::Provide)]
    nonce_mode: NonceMode,

    /// Seed for deterministic client-side behavior (currently influences only reporting metadata)
    #[arg(long)]
    seed: Option<u64>,

    /// Summary report JSON path
    #[arg(long, default_value = "out/txload_report.json")]
    report: PathBuf,

    /// Per-request events JSONL path (one JSON object per line)
    #[arg(long, default_value = "out/txload_events.jsonl")]
    events: PathBuf,
}

#[derive(Debug, Serialize)]
struct TxEvent {
    seq: u64,
    nonce: u64,
    start_ms: u64,
    end_ms: u64,
    latency_ms: f64,
    http_status: Option<u16>,
    rpc_error_code: Option<String>,
    rpc_error_message: Option<String>,
    tx_hash: Option<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    rpc: String,
    tps_target: u64,
    duration_seconds: u64,
    concurrency: usize,
    nonce_mode: String,
    total_planned: u64,
    total_sent: u64,
    accepted: u64,
    rejected: u64,
    achieved_tps_sent: f64,
    achieved_tps_accepted: f64,
    latency_ms_p50: f64,
    latency_ms_p95: f64,
    latency_ms_p99: f64,
    errors_by_http_status: BTreeMap<String, u64>,
    errors_by_rpc_code: BTreeMap<String, u64>,
    sample_tx_hashes: Vec<String>,
    from_address: String,
    to: String,
    memo: String,
    seed: Option<u64>,
    started_at_ms: u64,
    ended_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    nonce: u64,
}

#[derive(Debug, Deserialize)]
struct NonceEndpointResponse {
    nonce: u64,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaymentOk {
    tx_hash: Option<String>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

async fn fetch_next_nonce(client: &reqwest::Client, rpc: &str, sender_public_key_hex: &str) -> Result<u64> {
    // Prefer the fast nonce endpoint (no transaction history, no heavy scans).
    let url = format!("{}/nonce/{}", rpc.trim_end_matches('/'), sender_public_key_hex);
    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;

    // If the node doesn't have /nonce yet, fall back to /account.
    if resp.status().as_u16() == 404 {
        let url = format!("{}/account/{}", rpc.trim_end_matches('/'), sender_public_key_hex);
        let resp = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        if resp.status().as_u16() == 404 {
            return Err(anyhow!(
                "sender account not found on RPC ({}). Fund the sender first.",
                sender_public_key_hex
            ));
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("failed to fetch account nonce (status {status}): {body}"));
        }
        let json: AccountResponse = resp.json().await.context("parse /account response")?;
        return Ok(json.nonce.saturating_add(1));
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("failed to fetch account nonce (status {status}): {body}"));
    }

    let json: NonceEndpointResponse = resp.json().await.context("parse /nonce response")?;
    Ok(json.nonce.saturating_add(1))
}

async fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    if args.tps == 0 {
        return Err(anyhow!("--tps must be > 0"));
    }
    if args.duration == 0 {
        return Err(anyhow!("--duration must be > 0"));
    }
    if args.concurrency == 0 {
        return Err(anyhow!("--concurrency must be > 0"));
    }

    let rpc = args.rpc.trim_end_matches('/').to_string();
    let payment_url = format!("{rpc}/tx/payment");

    let password = args
        .from_key_password
        .clone()
        .or_else(|| std::env::var("IPPAN_KEY_PASSWORD").ok());

    let keyfile = KeyFile::load(&args.from_key)
        .with_context(|| format!("load keyfile {}", args.from_key.display()))?;
    let unlocked = keyfile
        .unlock(password.as_deref())
        .context("unlock keyfile")?;

    let from_address = unlocked.address.clone();
    let from_public_key_hex = hex::encode(unlocked.public_key);
    let signing_key_hex = hex::encode(unlocked.private_key);

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(args.concurrency.max(32))
        .build()
        .context("build reqwest client")?;

    let nonce_start = match args.nonce_mode {
        NonceMode::Omit => 0,
        NonceMode::Provide => match args.nonce_start {
            Some(n) => n,
            None => fetch_next_nonce(&client, &rpc, &from_public_key_hex).await?,
        },
    };

    let total_planned = args.tps.saturating_mul(args.duration);
    if total_planned == 0 {
        return Err(anyhow!("computed total planned tx is 0 (tps*duration overflow or zero)"));
    }

    if let Some(parent) = args.events.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let events_file = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&args.events)
        .await
        .with_context(|| format!("open events file {}", args.events.display()))?;

    // Event writer is single-threaded via channel to keep event ordering stable by seq.
    let (event_tx, mut event_rx) = mpsc::channel::<TxEvent>(50_000);
    let mut events_file_writer = events_file;
    let writer_task = tokio::spawn(async move {
        while let Some(ev) = event_rx.recv().await {
            if let Ok(mut line) = serde_json::to_vec(&ev) {
                line.push(b'\n');
                if events_file_writer.write_all(&line).await.is_err() {
                    break;
                }
            }
        }
        let _ = events_file_writer.flush().await;
    });

    // Pacer: send "tokens" (sequence numbers) at target TPS using tokio interval (fixed).
    let (token_tx, mut token_rx) = mpsc::channel::<u64>(args.tps.min(50_000) as usize);
    let pacer_tps = args.tps;
    let pacer_duration = Duration::from_secs(args.duration);
    let started_at_ms = now_ms();

    let pacer = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let start = tokio::time::Instant::now();
        let mut sent: u64 = 0;
        let mut remainder: u64 = 0;

        while tokio::time::Instant::now().duration_since(start) < pacer_duration {
            interval.tick().await;

            // Add tps/1000 tokens per 1ms tick, carry remainder.
            let base = pacer_tps / 1000;
            remainder += pacer_tps % 1000;
            let extra = if remainder >= 1000 {
                remainder -= 1000;
                1
            } else {
                0
            };
            let to_send = base + extra;

            for _ in 0..to_send {
                if sent >= pacer_tps.saturating_mul(pacer_duration.as_secs()) {
                    break;
                }
                if token_tx.send(sent).await.is_err() {
                    return sent;
                }
                sent += 1;
            }
        }
        sent
    });

    let sem = Arc::new(Semaphore::new(args.concurrency));
    let next_nonce = Arc::new(AtomicU64::new(nonce_start));

    let mut latency_hist = Histogram::<u64>::new_with_bounds(1, 60_000, 3)
        .context("init latency histogram")?;
    let mut errors_by_http_status: BTreeMap<String, u64> = BTreeMap::new();
    let mut errors_by_rpc_code: BTreeMap<String, u64> = BTreeMap::new();
    let mut accepted: u64 = 0;
    let mut rejected: u64 = 0;
    let mut sample_tx_hashes: Vec<String> = Vec::new();

    // We keep a bounded number of join handles to avoid unbounded memory.
    let mut in_flight: tokio::task::JoinSet<(bool, Option<u16>, Option<String>, Option<String>, Option<String>, u64)> =
        tokio::task::JoinSet::new();

    while let Some(seq) = token_rx.recv().await {
        // Drain completions opportunistically to keep joinset bounded.
        while in_flight.len() >= args.concurrency.saturating_mul(2) {
            if let Some(res) = in_flight.join_next().await {
                if let Ok((ok, status, rpc_code, _rpc_msg, tx_hash, latency_ms_u64)) = res {
                    if ok {
                        accepted += 1;
                        let _ = latency_hist.record(latency_ms_u64.max(1));
                        if let Some(h) = tx_hash {
                            if sample_tx_hashes.len() < 20 {
                                sample_tx_hashes.push(h);
                            }
                        }
                    } else {
                        rejected += 1;
                        if let Some(s) = status {
                            *errors_by_http_status.entry(s.to_string()).or_insert(0) += 1;
                        }
                        if let Some(code) = rpc_code {
                            *errors_by_rpc_code.entry(code).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let permit = sem.clone().acquire_owned().await?;
        let client = client.clone();
        let event_tx = event_tx.clone();
        let payment_url = payment_url.clone();
        let from = from_address.clone();
        let to = args.to.clone();
        let signing_key = signing_key_hex.clone();
        let memo = args.memo.clone();
        let fee = args.fee;
        let amount = args.amount;
        let nonce = match args.nonce_mode {
            NonceMode::Provide => next_nonce.fetch_add(1, Ordering::Relaxed),
            NonceMode::Omit => 0,
        };
        let include_nonce = args.nonce_mode == NonceMode::Provide;

        in_flight.spawn(async move {
            let _permit = permit;
            let start_ms = now_ms();

            let mut payload = serde_json::Map::new();
            payload.insert("from".into(), serde_json::Value::String(from));
            payload.insert("to".into(), serde_json::Value::String(to));
            payload.insert("amount".into(), serde_json::Value::String(amount.to_string()));
            payload.insert("memo".into(), serde_json::Value::String(memo));
            if include_nonce {
                payload.insert("nonce".into(), serde_json::Value::Number(nonce.into()));
            }
            payload.insert("signing_key".into(), serde_json::Value::String(signing_key));
            if let Some(fee_limit) = fee {
                payload.insert("fee".into(), serde_json::Value::String(fee_limit.to_string()));
            }

            let resp = client.post(&payment_url).json(&payload).send().await;
            let end_ms = now_ms();
            let latency_ms = (end_ms.saturating_sub(start_ms)) as f64;

            let mut http_status: Option<u16> = None;
            let mut rpc_error_code: Option<String> = None;
            let mut rpc_error_message: Option<String> = None;
            let mut tx_hash: Option<String> = None;
            let mut ok = false;

            match resp {
                Ok(r) => {
                    http_status = Some(r.status().as_u16());
                    if r.status().is_success() {
                        if let Ok(body) = r.json::<PaymentOk>().await {
                            tx_hash = body.tx_hash;
                        }
                        ok = true;
                    } else if let Ok(err_body) = r.json::<ApiError>().await {
                        rpc_error_code = err_body.code;
                        rpc_error_message = err_body.message;
                    }
                }
                Err(e) => {
                    rpc_error_code = Some("client_error".to_string());
                    rpc_error_message = Some(e.to_string());
                }
            }

            let _ = event_tx
                .send(TxEvent {
                    seq,
                    nonce,
                    start_ms,
                    end_ms,
                    latency_ms,
                    http_status,
                    rpc_error_code: rpc_error_code.clone(),
                    rpc_error_message: rpc_error_message.clone(),
                    tx_hash: tx_hash.clone(),
                })
                .await;

            (
                ok,
                http_status,
                rpc_error_code,
                rpc_error_message,
                tx_hash,
                latency_ms.max(1.0) as u64,
            )
        });
    }

    // Wait for pacer and remaining inflight.
    let total_sent = pacer.await.unwrap_or(0);
    drop(event_tx);

    while let Some(res) = in_flight.join_next().await {
        if let Ok((ok, status, rpc_code, _rpc_msg, tx_hash, latency_ms_u64)) = res {
            if ok {
                accepted += 1;
                let _ = latency_hist.record(latency_ms_u64.max(1));
                if let Some(h) = tx_hash {
                    if sample_tx_hashes.len() < 20 {
                        sample_tx_hashes.push(h);
                    }
                }
            } else {
                rejected += 1;
                if let Some(s) = status {
                    *errors_by_http_status.entry(s.to_string()).or_insert(0) += 1;
                }
                if let Some(code) = rpc_code {
                    *errors_by_rpc_code.entry(code).or_insert(0) += 1;
                }
            }
        }
    }

    let _ = writer_task.await;

    let ended_at_ms = now_ms();
    let elapsed_s = (ended_at_ms.saturating_sub(started_at_ms) as f64 / 1000.0).max(0.001);
    let achieved_tps_sent = total_sent as f64 / elapsed_s;
    let achieved_tps_accepted = accepted as f64 / elapsed_s;

    let latency_ms_p50 = if latency_hist.len() == 0 {
        0.0
    } else {
        latency_hist.value_at_quantile(0.50) as f64
    };
    let latency_ms_p95 = if latency_hist.len() == 0 {
        0.0
    } else {
        latency_hist.value_at_quantile(0.95) as f64
    };
    let latency_ms_p99 = if latency_hist.len() == 0 {
        0.0
    } else {
        latency_hist.value_at_quantile(0.99) as f64
    };

    let report = Report {
        rpc: rpc.clone(),
        tps_target: args.tps,
        duration_seconds: args.duration,
        concurrency: args.concurrency,
        nonce_mode: match args.nonce_mode {
            NonceMode::Provide => "provide".to_string(),
            NonceMode::Omit => "omit".to_string(),
        },
        total_planned,
        total_sent,
        accepted,
        rejected,
        achieved_tps_sent,
        achieved_tps_accepted,
        latency_ms_p50,
        latency_ms_p95,
        latency_ms_p99,
        errors_by_http_status,
        errors_by_rpc_code,
        sample_tx_hashes,
        from_address,
        to: args.to.clone(),
        memo: args.memo.clone(),
        seed: args.seed,
        started_at_ms,
        ended_at_ms,
    };

    write_json(&args.report, &report).await?;

    println!("ippan-txload done");
    println!("  rpc: {}", rpc);
    println!("  total_sent: {}", total_sent);
    println!("  accepted: {}  rejected: {}", accepted, rejected);
    println!("  achieved_tps_sent: {:.2}", achieved_tps_sent);
    println!("  achieved_tps_accepted: {:.2}", achieved_tps_accepted);
    println!("  latency_ms p50/p95/p99: {:.2} / {:.2} / {:.2}", latency_ms_p50, latency_ms_p95, latency_ms_p99);
    println!("  report: {}", args.report.display());
    println!("  events: {}", args.events.display());

    Ok(())
}


