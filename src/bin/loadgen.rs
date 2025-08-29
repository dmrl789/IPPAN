use clap::Parser;
use ippan::{crypto::KeyPair, transaction::Transaction, time::IppanTime};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

#[derive(Parser)]
#[command(name = "ippan-loadgen")]
#[command(about = "IPPAN Load Generator")]
struct Cli {
    /// Target transactions per second
    #[arg(long, default_value = "1000")]
    tps: u64,
    
    /// Number of accounts to use
    #[arg(long, default_value = "10")]
    accounts: usize,
    
    /// Duration in seconds
    #[arg(long, default_value = "60")]
    duration: u64,
    
    /// Node URLs (comma-separated)
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    nodes: String,
    
    /// Number of shards
    #[arg(long, default_value = "1")]
    shards: usize,
}

struct LoadGenerator {
    tps: u64,
    accounts: Vec<KeyPair>,
    nodes: Vec<String>,
    ippan_time: Arc<IppanTime>,
    client: reqwest::Client,
}

impl LoadGenerator {
    fn new(tps: u64, account_count: usize, nodes: Vec<String>) -> Self {
        let accounts: Vec<KeyPair> = (0..account_count)
            .map(|_| KeyPair::generate())
            .collect();
        
        Self {
            tps,
            accounts,
            nodes,
            ippan_time: Arc::new(IppanTime::new()),
            client: reqwest::Client::new(),
        }
    }
    
    async fn run(&self, duration_secs: u64) -> anyhow::Result<()> {
        println!("Starting load generator:");
        println!("  TPS: {}", self.tps);
        println!("  Accounts: {}", self.accounts.len());
        println!("  Duration: {}s", duration_secs);
        println!("  Nodes: {}", self.nodes.join(", "));
        
        let start_time = Instant::now();
        let end_time = start_time + Duration::from_secs(duration_secs);
        
        let interval_duration = Duration::from_millis(1000 / self.tps);
        let mut interval = interval(interval_duration);
        
        let mut total_sent = 0u64;
        let mut total_success = 0u64;
        let mut total_failed = 0u64;
        
        while Instant::now() < end_time {
            interval.tick().await;
            
            // Create a transaction
            let from_account = &self.accounts[total_sent as usize % self.accounts.len()];
            let to_account = &self.accounts[(total_sent as usize + 1) % self.accounts.len()];
            
            let transaction = Transaction::new(
                from_account,
                to_account.public_key,
                1, // Send 1 unit
                (total_sent / self.accounts.len() as u64) + 1, // Nonce
                self.ippan_time.clone(),
            )?;
            
            // Serialize and encode transaction
            let tx_data = transaction.serialize()?;
            let tx_hex = hex::encode(&tx_data);
            
            // Submit to a random node
            let node_url = &self.nodes[total_sent as usize % self.nodes.len()];
            let response = self.client
                .post(&format!("{}/tx", node_url))
                .json(&serde_json::json!({
                    "transaction": tx_hex
                }))
                .send()
                .await;
            
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(result) = resp.json::<serde_json::Value>().await {
                            if result["success"].as_bool().unwrap_or(false) {
                                total_success += 1;
                            } else {
                                total_failed += 1;
                                eprintln!("Transaction failed: {}", result["message"].as_str().unwrap_or("Unknown error"));
                            }
                        } else {
                            total_failed += 1;
                        }
                    } else {
                        total_failed += 1;
                        eprintln!("HTTP error: {}", resp.status());
                    }
                }
                Err(e) => {
                    total_failed += 1;
                    eprintln!("Request failed: {}", e);
                }
            }
            
            total_sent += 1;
            
            // Print progress every 1000 transactions
            if total_sent % 1000 == 0 {
                let elapsed = start_time.elapsed();
                let actual_tps = total_sent as f64 / elapsed.as_secs_f64();
                println!("Progress: {} tx sent, {:.1} TPS, {} success, {} failed", 
                         total_sent, actual_tps, total_success, total_failed);
            }
        }
        
        let total_elapsed = start_time.elapsed();
        let final_tps = total_sent as f64 / total_elapsed.as_secs_f64();
        let success_rate = (total_success as f64 / total_sent as f64) * 100.0;
        
        println!("\nLoad test completed:");
        println!("  Total transactions: {}", total_sent);
        println!("  Successful: {}", total_success);
        println!("  Failed: {}", total_failed);
        println!("  Success rate: {:.1}%", success_rate);
        println!("  Average TPS: {:.1}", final_tps);
        println!("  Duration: {:.1}s", total_elapsed.as_secs_f64());
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let nodes: Vec<String> = cli.nodes
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    
    let loadgen = LoadGenerator::new(cli.tps, cli.accounts, nodes);
    loadgen.run(cli.duration).await?;
    
    Ok(())
}
