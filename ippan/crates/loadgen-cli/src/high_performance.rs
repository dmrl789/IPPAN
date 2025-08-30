use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::time::{Duration, Instant};
use anyhow::{Result, Context};
use reqwest::Client;
use ippan_common::{Transaction, KeyPair, time::ippan_time_us, crypto::hashtimer};
use serde::Serialize;

/// High-performance load generator for 10M+ TPS testing
pub struct HighPerformanceLoadGenerator {
    accounts: Vec<KeyPair>,
    node_urls: Vec<String>,
    client_pool: Vec<Client>,
    semaphore: Arc<Semaphore>,
    results: Arc<RwLock<LoadTestResults>>,
    batch_size: usize,
    concurrency_limit: usize,
}

// Use the same LoadTestResults type as the main module
use crate::LoadTestResults;

impl HighPerformanceLoadGenerator {
    pub fn new(
        account_count: usize,
        node_urls: Vec<String>,
        concurrency_limit: usize,
        batch_size: usize,
    ) -> Self {
        // Generate accounts
        let accounts: Vec<KeyPair> = (0..account_count)
            .map(|_| KeyPair::generate())
            .collect();

        // Create client pool
        let client_pool: Vec<Client> = (0..concurrency_limit)
            .map(|_| {
                Client::builder()
                    .timeout(Duration::from_secs(30))
                    .pool_max_idle_per_host(100)
                    .pool_idle_timeout(Duration::from_secs(90))
                    .build()
                    .unwrap()
            })
            .collect();

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
            node_urls,
            client_pool,
            semaphore: Arc::new(Semaphore::new(concurrency_limit)),
            results: Arc::new(RwLock::new(results)),
            batch_size,
            concurrency_limit,
        }
    }

    /// Run high-performance load test
    pub async fn run_load_test(&self, target_tps: u64, duration: Duration) -> Result<LoadTestResults> {
        let start_time = Instant::now();
        let mut latencies = Arc::new(RwLock::new(Vec::new()));
        
        println!("🚀 Starting High-Performance Load Test:");
        println!("   Target TPS: {}", target_tps);
        println!("   Duration: {} seconds", duration.as_secs());
        println!("   Accounts: {}", self.accounts.len());
        println!("   Nodes: {}", self.node_urls.len());
        println!("   Concurrency: {}", self.concurrency_limit);
        println!("   Batch Size: {}", self.batch_size);

        // Calculate batch interval
        let batch_interval = Duration::from_millis(1000) / (target_tps / self.batch_size as u64) as u32;
        
        let mut last_report = Instant::now();
        let mut batch_count = 0;

        while start_time.elapsed() < duration {
            let batch_start = Instant::now();
            
            // Send batch of transactions
            let batch_tasks: Vec<_> = (0..self.batch_size)
                .map(|_| {
                    let generator = self.clone_for_task();
                    let latencies = latencies.clone();
                    tokio::spawn(async move {
                        let _permit = generator.semaphore.acquire().await.unwrap();
                        generator.send_single_transaction(latencies).await
                    })
                })
                .collect();

            // Wait for batch to complete
            let results = futures::future::join_all(batch_tasks).await;
            
            // Update counters
            let mut results_guard = self.results.write().await;
            for result in results {
                match result {
                    Ok(Ok(_)) => {
                        results_guard.successful_transactions += 1;
                        results_guard.total_transactions += 1;
                    }
                    _ => {
                        results_guard.failed_transactions += 1;
                        results_guard.total_transactions += 1;
                    }
                }
            }
            drop(results_guard);

            batch_count += 1;

            // Progress reporting
            if last_report.elapsed() >= Duration::from_secs(5) {
                let elapsed = start_time.elapsed().as_secs();
                let current_tps = (batch_count * self.batch_size) as f64 / elapsed as f64;
                println!("   Progress: {}s, TPS: {:.0}, Batches: {}", elapsed, current_tps, batch_count);
                last_report = Instant::now();
            }

            // Rate limiting
            if batch_start.elapsed() < batch_interval {
                tokio::time::sleep(batch_interval - batch_start.elapsed()).await;
            }
        }

        // Calculate final results
        let total_duration = start_time.elapsed();
        let mut results_guard = self.results.write().await;
        results_guard.total_duration_seconds = total_duration.as_secs_f64();
        results_guard.target_tps = target_tps;
        results_guard.actual_tps = results_guard.successful_transactions as f64 / results_guard.total_duration_seconds;

        // Calculate latency statistics
        let latencies_guard = latencies.read().await;
        if !latencies_guard.is_empty() {
            let mut sorted_latencies = latencies_guard.clone();
            sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let len = sorted_latencies.len();

            results_guard.average_latency_ms = sorted_latencies.iter().sum::<f64>() / len as f64;
            results_guard.p50_latency_ms = sorted_latencies[len * 50 / 100];
            results_guard.p95_latency_ms = sorted_latencies[len * 95 / 100];
            results_guard.p99_latency_ms = sorted_latencies[len * 99 / 100];
        }

        Ok(results_guard.clone())
    }

    /// Send a single transaction
    async fn send_single_transaction(&self, latencies: Arc<RwLock<Vec<f64>>>) -> Result<()> {
        let start = Instant::now();

        // Generate transaction
        let tx = self.generate_transaction()?;
        let tx_data = bincode::serialize(&tx)
            .context("Failed to serialize transaction")?;

        // Select random client and node
        let client_index = rand::random::<usize>() % self.client_pool.len();
        let node_index = rand::random::<usize>() % self.node_urls.len();
        
        let client = &self.client_pool[client_index];
        let node_url = &self.node_urls[node_index];
        let url = format!("{}/tx", node_url);

        // Send transaction
        let response = client
            .post(&url)
            .body(tx_data)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await
            .context("Failed to send transaction")?;

        let latency = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            let error = response.text().await
                .context("Failed to read error response")?;
            anyhow::bail!("Transaction failed: {}", error);
        }

        // Record latency
        let mut latencies_guard = latencies.write().await;
        latencies_guard.push(latency);

        Ok(())
    }

    /// Generate a transaction
    fn generate_transaction(&self) -> Result<Transaction> {
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
            from_pub: account.public_key,
            to_addr: recipient.public_key,
            amount: 1, // Send 1 IPPAN
            nonce: 0, // Simplified for high-TPS testing
            ippan_time_us: ippan_time,
            hashtimer,
            sig: [0u8; 64], // Will be set after signing
        };
        
        // Sign transaction
        tx.sig = account.sign(&tx.message_to_sign()?)?;
        
        Ok(tx)
    }

    /// Clone for async tasks
    fn clone_for_task(&self) -> Self {
        Self {
            accounts: self.accounts.clone(),
            node_urls: self.node_urls.clone(),
            client_pool: self.client_pool.clone(),
            semaphore: self.semaphore.clone(),
            results: self.results.clone(),
            batch_size: self.batch_size,
            concurrency_limit: self.concurrency_limit,
        }
    }
}
