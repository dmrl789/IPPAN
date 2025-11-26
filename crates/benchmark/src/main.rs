//! IPPAN Performance Benchmarking Tool
//!
//! Measure node performance, TPS, latency, and network throughput.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "ippan-benchmark")]
#[command(about = "IPPAN Performance Benchmarking Tool")]
#[command(version)]
struct Cli {
    /// RPC endpoint URL
    #[arg(long, default_value = "http://localhost:8080")]
    rpc_url: String,

    /// Number of transactions/operations
    #[arg(short, long, default_value = "1000")]
    count: usize,

    /// Number of concurrent workers
    #[arg(short, long, default_value = "10")]
    workers: usize,

    /// Test type
    #[arg(short, long, value_enum, default_value = "tps")]
    test: TestType,

    /// Warmup iterations before actual test
    #[arg(long, default_value = "10")]
    warmup: usize,
}

#[derive(ValueEnum, Clone, Debug)]
enum TestType {
    /// Transactions per second
    Tps,
    /// Block production latency
    Latency,
    /// P2P network throughput
    Network,
    /// Storage read/write performance
    Storage,
    /// Full benchmark suite
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("╔════════════════════════════════════════╗");
    println!("║  IPPAN Blockchain Benchmark Tool      ║");
    println!("╚════════════════════════════════════════╝\n");

    let cli = Cli::parse();

    // Check node connectivity
    println!("🔍 Checking node connectivity...");
    check_node(&cli.rpc_url).await?;
    println!("✓ Connected to IPPAN node\n");

    match cli.test {
        TestType::Tps => benchmark_tps(&cli).await?,
        TestType::Latency => benchmark_latency(&cli).await?,
        TestType::Network => benchmark_network(&cli).await?,
        TestType::Storage => benchmark_storage(&cli).await?,
        TestType::All => {
            benchmark_tps(&cli).await?;
            println!("\n");
            benchmark_latency(&cli).await?;
            println!("\n");
            benchmark_network(&cli).await?;
            println!("\n");
            benchmark_storage(&cli).await?;
        }
    }

    Ok(())
}

async fn check_node(rpc_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    client.get(format!("{rpc_url}/node/status")).send().await?;
    Ok(())
}

async fn benchmark_tps(cli: &Cli) -> Result<()> {
    println!("📊 Running TPS (Transactions Per Second) Benchmark");
    println!("═══════════════════════════════════════════════════");
    println!("Transactions: {}", cli.count);
    println!("Workers:      {}", cli.workers);
    println!();

    // Warmup
    if cli.warmup > 0 {
        print!("Warming up ({} iterations)... ", cli.warmup);
        warmup_requests(&cli.rpc_url, cli.warmup).await?;
        println!("✓");
    }

    let client = reqwest::Client::new();
    let start = Instant::now();

    let mut handles = vec![];
    let tx_per_worker = cli.count / cli.workers;

    println!("Sending transactions...");

    for worker_id in 0..cli.workers {
        let client = client.clone();
        let rpc_url = cli.rpc_url.clone();

        let handle = tokio::spawn(async move {
            let mut success_count = 0;
            let mut error_count = 0;

            for i in 0..tx_per_worker {
                let tx = serde_json::json!({
                    "from": format!("benchmark_worker_{}", worker_id),
                    "to": format!("benchmark_receiver_{}", i % 100),
                    "amount": 1,
                    "timestamp": chrono::Utc::now().timestamp_micros(),
                    "nonce": i,
                });

                match client
                    .post(format!("{rpc_url}/transaction"))
                    .json(&tx)
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(_) => success_count += 1,
                    Err(_) => error_count += 1,
                }
            }

            (success_count, error_count)
        });

        handles.push(handle);
    }

    let mut total_success = 0;
    let mut total_errors = 0;

    for handle in handles {
        let (success, errors) = handle.await?;
        total_success += success;
        total_errors += errors;
    }

    let duration = start.elapsed();
    let tps = total_success as f64 / duration.as_secs_f64();

    println!();
    println!("╔═══════════════════════════════════════╗");
    println!("║           TPS Results                 ║");
    println!("╠═══════════════════════════════════════╣");
    println!("║ Total transactions:    {:>14} ║", cli.count);
    println!("║ Successful:            {total_success:>14} ║");
    println!("║ Failed:                {total_errors:>14} ║");
    println!(
        "║ Total time:            {:>11.2}s ║",
        duration.as_secs_f64()
    );
    println!("║ TPS:                   {tps:>14.2} ║");
    println!(
        "║ Avg latency:           {:>11.2}ms ║",
        duration.as_millis() as f64 / total_success as f64
    );
    println!("╚═══════════════════════════════════════╝");

    Ok(())
}

async fn benchmark_latency(cli: &Cli) -> Result<()> {
    println!("⚡ Running Latency Benchmark");
    println!("════════════════════════════");
    println!("Requests: {}", cli.count);
    println!();

    // Warmup
    if cli.warmup > 0 {
        print!("Warming up... ");
        warmup_requests(&cli.rpc_url, cli.warmup).await?;
        println!("✓");
    }

    let client = reqwest::Client::new();
    let mut latencies = vec![];

    println!("Measuring latency...");

    for i in 0..cli.count {
        let start = Instant::now();

        let result = client
            .get(format!("{}/block/latest", cli.rpc_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        if result.is_ok() {
            let latency = start.elapsed();
            latencies.push(latency);
        }

        if i % 100 == 0 && i > 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }

    println!();

    if latencies.is_empty() {
        println!("⚠️  No successful requests");
        return Ok(());
    }

    latencies.sort();
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let min_latency = latencies.first().unwrap();
    let max_latency = latencies.last().unwrap();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];

    println!();
    println!("╔═══════════════════════════════════════╗");
    println!("║        Latency Results                ║");
    println!("╠═══════════════════════════════════════╣");
    println!("║ Requests:              {:>14} ║", latencies.len());
    println!(
        "║ Min:                   {:>11.2}ms ║",
        min_latency.as_secs_f64() * 1000.0
    );
    println!(
        "║ Avg:                   {:>11.2}ms ║",
        avg_latency.as_secs_f64() * 1000.0
    );
    println!(
        "║ Max:                   {:>11.2}ms ║",
        max_latency.as_secs_f64() * 1000.0
    );
    println!(
        "║ P50 (median):          {:>11.2}ms ║",
        p50.as_secs_f64() * 1000.0
    );
    println!(
        "║ P95:                   {:>11.2}ms ║",
        p95.as_secs_f64() * 1000.0
    );
    println!(
        "║ P99:                   {:>11.2}ms ║",
        p99.as_secs_f64() * 1000.0
    );
    println!("╚═══════════════════════════════════════╝");

    Ok(())
}

async fn benchmark_network(cli: &Cli) -> Result<()> {
    println!("🌐 Running Network Benchmark");
    println!("═══════════════════════════");
    println!();

    let client = reqwest::Client::new();
    let start = Instant::now();

    let resp: serde_json::Value = client
        .get(format!("{}/node/peers", cli.rpc_url))
        .send()
        .await?
        .json()
        .await?;

    let duration = start.elapsed();

    println!("╔═══════════════════════════════════════╗");
    println!("║       Network Stats                   ║");
    println!("╠═══════════════════════════════════════╣");
    println!(
        "║ Response time:         {:>11.2}ms ║",
        duration.as_secs_f64() * 1000.0
    );
    println!("╚═══════════════════════════════════════╝");
    println!();
    println!("Peer data:");
    println!("{}", serde_json::to_string_pretty(&resp)?);

    Ok(())
}

async fn benchmark_storage(cli: &Cli) -> Result<()> {
    println!("💾 Running Storage Benchmark");
    println!("════════════════════════════");
    println!("Queries: {}", cli.count);
    println!();

    // Warmup
    if cli.warmup > 0 {
        print!("Warming up... ");
        warmup_requests(&cli.rpc_url, cli.warmup).await?;
        println!("✓");
    }

    let client = reqwest::Client::new();
    let mut read_times = vec![];

    println!("Testing block queries...");

    for i in 0..cli.count {
        let start = Instant::now();

        let result = client
            .get(format!("{}/block/{}", cli.rpc_url, i % 100))
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        if result.is_ok() {
            read_times.push(start.elapsed());
        }

        if i % 100 == 0 && i > 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }

    println!();

    if read_times.is_empty() {
        println!("⚠️  No successful queries");
        return Ok(());
    }

    let avg_read = read_times.iter().sum::<Duration>() / read_times.len() as u32;
    let reads_per_sec = 1000.0 / (avg_read.as_secs_f64() * 1000.0);

    read_times.sort();
    let p50 = read_times[read_times.len() / 2];
    let p95 = read_times[(read_times.len() * 95) / 100];

    println!();
    println!("╔═══════════════════════════════════════╗");
    println!("║       Storage Results                 ║");
    println!("╠═══════════════════════════════════════╣");
    println!("║ Queries:               {:>14} ║", read_times.len());
    println!(
        "║ Avg read time:         {:>11.2}ms ║",
        avg_read.as_secs_f64() * 1000.0
    );
    println!(
        "║ P50 read time:         {:>11.2}ms ║",
        p50.as_secs_f64() * 1000.0
    );
    println!(
        "║ P95 read time:         {:>11.2}ms ║",
        p95.as_secs_f64() * 1000.0
    );
    println!("║ Reads/sec:             {reads_per_sec:>14.0} ║");
    println!("╚═══════════════════════════════════════╝");

    Ok(())
}

async fn warmup_requests(rpc_url: &str, count: usize) -> Result<()> {
    let client = reqwest::Client::new();

    for _ in 0..count {
        let _ = client
            .get(format!("{rpc_url}/block/latest"))
            .timeout(Duration::from_secs(2))
            .send()
            .await;
    }

    Ok(())
}
