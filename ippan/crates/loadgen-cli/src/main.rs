use clap::Parser;
use ippan_common::{Transaction, KeyPair, crypto::derive_address, time::ippan_time_us, crypto::hashtimer};
use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use anyhow::{Result, Context};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

/// IPPAN Load Generator CLI
#[derive(Parser)]
#[command(name = "ippan-loadgen")]
#[command(about = "IPPAN Load Generator - Generate high-TPS transaction load")]
struct Cli {
    /// Target TPS (transactions per second)
    #[arg(short, long, default_value = "1000")]
    tps: u64,
    
    /// Number of accounts to use
    #[arg(short, long, default_value = "100")]
    accounts: usize,
    
    /// Test duration in seconds
    #[arg(short, long, default_value = "60")]
    duration: u64,
    
    /// Node URLs (comma-separated)
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    nodes: String,
    
    /// Number of shards
    #[arg(short, long, default_value = "4")]
    shards: usize,
    
    /// Output CSV file for results
    #[arg(short, long)]
    output: Option<String>,
    
    /// Pre-fund accounts with initial balance
    #[arg(short, long, default_value = "1000000")]
    initial_balance: u64,
}

/// Load test account
#[derive(Debug, Clone)]
struct LoadAccount {
    keypair: KeyPair,
    address: String,
    balance: u64,
    nonce: u64,
}

/// Load test results
#[derive(Debug, Serialize, Clone)]
struct LoadTestResults {
    total_transactions: u64,
    successful_transactions: u64,
    failed_transactions: u64,
    total_duration_seconds: f64,
    actual_tps: f64,
    target_tps: u64,
    average_latency_ms: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    errors: Vec<String>,
}

/// Transaction sender with load balancing
struct LoadBalancedSender {
    clients: Vec<Client>,
    current_index: Arc<RwLock<usize>>,
}

impl LoadBalancedSender {
    fn new(node_urls: Vec<String>) -> Self {
        let clients = node_urls.into_iter()
            .map(|_| Client::new())
            .collect();
        
        Self {
            clients,
            current_index: Arc::new(RwLock::new(0)),
        }
    }
    
    async fn send_transaction(&self, tx: &Transaction) -> Result<Duration> {
        let start = Instant::now();
        
        let tx_data = bincode::serialize(tx)
            .context("Failed to serialize transaction")?;
        
        // Round-robin load balancing
        let mut index = self.current_index.write().await;
        let client = &self.clients[*index];
        *index = (*index + 1) % self.clients.len();
        drop(index);
        
        let response = client
            .post("/tx")
            .body(tx_data)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await
            .context("Failed to send transaction")?;
        
        let latency = start.elapsed();
        
        if !response.status().is_success() {
            let error = response.text().await
                .context("Failed to read error response")?;
            anyhow::bail!("Transaction failed: {}", error);
        }
        
        Ok(latency)
    }
}

/// Load generator
struct LoadGenerator {
    accounts: Vec<LoadAccount>,
    sender: LoadBalancedSender,
    results: Arc<RwLock<LoadTestResults>>,
}

impl LoadGenerator {
    fn new(accounts: Vec<LoadAccount>, node_urls: Vec<String>) -> Self {
        let results = LoadTestResults {
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            total_duration_seconds: 0.0,
            actual_tps: 0.0,
            target_tps: 0,
            average_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            errors: Vec::new(),
        };
        
        Self {
            accounts,
            sender: LoadBalancedSender::new(node_urls),
            results: Arc::new(RwLock::new(results)),
        }
    }
    
    async fn run_load_test(&mut self, target_tps: u64, duration: Duration) -> Result<LoadTestResults> {
        let start_time = Instant::now();
        let interval = Duration::from_millis(1000) / target_tps as u32;
        let mut latencies = Vec::new();
        
        println!("🚀 Starting load test:");
        println!("   Target TPS: {}", target_tps);
        println!("   Duration: {} seconds", duration.as_secs());
        println!("   Accounts: {}", self.accounts.len());
        println!("   Interval: {:?}", interval);
        
        let mut last_report = Instant::now();
        let mut tx_count = 0;
        
        while start_time.elapsed() < duration {
            let tx_start = Instant::now();
            
            // Generate and send transaction
            if let Ok(latency) = self.generate_and_send_transaction().await {
                latencies.push(latency.as_millis() as f64);
                tx_count += 1;
                
                // Update results
                let mut results = self.results.write().await;
                results.successful_transactions += 1;
                results.total_transactions += 1;
            } else {
                let mut results = self.results.write().await;
                results.failed_transactions += 1;
                results.total_transactions += 1;
            }
            
            // Progress reporting
            if last_report.elapsed() >= Duration::from_secs(5) {
                let elapsed = start_time.elapsed().as_secs();
                let current_tps = tx_count as f64 / elapsed as f64;
                println!("   Progress: {}s, TPS: {:.1}, Total: {}", elapsed, current_tps, tx_count);
                last_report = Instant::now();
            }
            
            // Rate limiting
            if tx_start.elapsed() < interval {
                sleep(interval - tx_start.elapsed()).await;
            }
        }
        
        // Calculate final results
        let total_duration = start_time.elapsed();
        let mut results = self.results.write().await;
        results.total_duration_seconds = total_duration.as_secs_f64();
        results.target_tps = target_tps;
        results.actual_tps = results.successful_transactions as f64 / results.total_duration_seconds;
        
        // Calculate latency statistics
        if !latencies.is_empty() {
            latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let len = latencies.len();
            
            results.average_latency_ms = latencies.iter().sum::<f64>() / len as f64;
            results.p50_latency_ms = latencies[len * 50 / 100];
            results.p95_latency_ms = latencies[len * 95 / 100];
            results.p99_latency_ms = latencies[len * 99 / 100];
        }
        
        Ok(results.clone())
    }
    
    async fn generate_and_send_transaction(&self) -> Result<Duration> {
        // Select random account
        let account_index = rand::random::<usize>() % self.accounts.len();
        let account = &self.accounts[account_index];
        
        // Select random recipient (different from sender)
        let recipient_index = (account_index + 1 + rand::random::<usize>() % (self.accounts.len() - 1)) % self.accounts.len();
        let recipient = &self.accounts[recipient_index];
        
        // Create transaction
        let ippan_time = ippan_time_us();
        let tx_id = [0u8; 32]; // Placeholder
        let hashtimer = hashtimer(&tx_id);
        
        let mut tx = Transaction {
            ver: 1,
            from_pub: account.keypair.public_key,
            to_addr: recipient.keypair.public_key,
            amount: 1, // Send 1 IPPAN
            nonce: account.nonce,
            ippan_time_us: ippan_time,
            hashtimer,
            sig: [0u8; 64], // Will be set after signing
        };
        
        // Sign transaction
        tx.sig = account.keypair.sign(&bincode::serialize(&tx)?)?;
        
        // Send transaction
        self.sender.send_transaction(&tx).await
    }
    
    async fn pre_fund_accounts(&self, initial_balance: u64) -> Result<()> {
        println!("💰 Pre-funding {} accounts with {} IPPAN each...", 
            self.accounts.len(), initial_balance);
        
        // TODO: Implement account pre-funding
        // This would typically involve:
        // 1. Creating genesis transactions
        // 2. Sending them to the node
        // 3. Waiting for confirmation
        
        println!("   (Pre-funding not implemented in this MVP - using placeholder balances)");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Parse node URLs
    let node_urls: Vec<String> = cli.nodes
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
    if node_urls.is_empty() {
        anyhow::bail!("No node URLs provided");
    }
    
    println!("🎯 IPPAN Load Generator");
    println!("   Nodes: {}", node_urls.join(", "));
    println!("   Target TPS: {}", cli.tps);
    println!("   Accounts: {}", cli.accounts);
    println!("   Duration: {}s", cli.duration);
    println!("   Shards: {}", cli.shards);
    
    // Generate test accounts
    println!("🔑 Generating {} test accounts...", cli.accounts);
    let mut accounts = Vec::new();
    
    for _i in 0..cli.accounts {
        let keypair = KeyPair::generate();
        let address = derive_address(&keypair.public_key);
        
        accounts.push(LoadAccount {
            keypair,
            address,
            balance: cli.initial_balance,
            nonce: 0,
        });
    }
    
    // Create load generator
    let mut generator = LoadGenerator::new(accounts, node_urls);
    
    // Pre-fund accounts
    generator.pre_fund_accounts(cli.initial_balance).await?;
    
    // Run load test
    let duration = Duration::from_secs(cli.duration);
    let results = generator.run_load_test(cli.tps, duration).await?;
    
    // Print results
    println!("\n📊 Load Test Results:");
    println!("   Total Transactions: {}", results.total_transactions);
    println!("   Successful: {}", results.successful_transactions);
    println!("   Failed: {}", results.failed_transactions);
    println!("   Success Rate: {:.2}%", 
        (results.successful_transactions as f64 / results.total_transactions as f64) * 100.0);
    println!("   Target TPS: {}", results.target_tps);
    println!("   Actual TPS: {:.2}", results.actual_tps);
    println!("   Duration: {:.2}s", results.total_duration_seconds);
    println!("   Average Latency: {:.2}ms", results.average_latency_ms);
    println!("   P50 Latency: {:.2}ms", results.p50_latency_ms);
    println!("   P95 Latency: {:.2}ms", results.p95_latency_ms);
    println!("   P99 Latency: {:.2}ms", results.p99_latency_ms);
    
    if !results.errors.is_empty() {
        println!("   Errors: {}", results.errors.len());
        for error in results.errors.iter().take(5) {
            println!("     - {}", error);
        }
    }
    
    // Save results to CSV if requested
    if let Some(output_path) = cli.output {
        let csv_data = format!(
            "total_transactions,successful_transactions,failed_transactions,total_duration_seconds,actual_tps,target_tps,average_latency_ms,p50_latency_ms,p95_latency_ms,p99_latency_ms\n{},{},{},{},{},{},{},{},{},{}\n",
            results.total_transactions,
            results.successful_transactions,
            results.failed_transactions,
            results.total_duration_seconds,
            results.actual_tps,
            results.target_tps,
            results.average_latency_ms,
            results.p50_latency_ms,
            results.p95_latency_ms,
            results.p99_latency_ms
        );
        
        std::fs::write(&output_path, csv_data)
            .context(format!("Failed to write results to {}", output_path))?;
        
        println!("   Results saved to: {}", output_path);
    }
    
    Ok(())
}
