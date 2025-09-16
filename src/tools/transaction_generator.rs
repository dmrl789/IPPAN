//! Transaction generator for stress testing the IPPAN network
//! 
//! Generates thousands of transactions between nodes to test real blockchain functionality

use crate::{Result, IppanError};
use crate::transaction::{Transaction, TransactionType, create_transaction};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Transaction generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Number of transactions to generate per second
    pub tx_per_second: u32,
    /// Number of accounts to use
    pub num_accounts: usize,
    /// Initial balance for each account
    pub initial_balance: u64,
    /// Transaction amount range (min, max)
    pub amount_range: (u64, u64),
    /// Transaction fee range (min, max)
    pub fee_range: (u64, u64),
    /// Duration to run the generator (in seconds)
    pub duration_seconds: u64,
    /// Target node URLs
    pub target_nodes: Vec<String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            tx_per_second: 10,
            num_accounts: 100,
            initial_balance: 1000000, // 1M tokens per account
            amount_range: (100, 10000),
            fee_range: (1, 100),
            duration_seconds: 300, // 5 minutes
            target_nodes: vec![
                "http://188.245.97.41:3000".to_string(),
                "http://135.181.145.174:3001".to_string(),
            ],
        }
    }
}

/// Account state for transaction generation
#[derive(Debug, Clone)]
struct AccountState {
    address: String,
    balance: u64,
    nonce: u64,
}

/// Transaction generator
pub struct TransactionGenerator {
    config: GeneratorConfig,
    accounts: Arc<RwLock<Vec<AccountState>>>,
    http_client: reqwest::Client,
    stats: Arc<RwLock<GeneratorStats>>,
}

/// Generator statistics
#[derive(Debug, Default, Clone)]
pub struct GeneratorStats {
    pub total_sent: u64,
    pub total_successful: u64,
    pub total_failed: u64,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
}

impl TransactionGenerator {
    /// Create a new transaction generator
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            config,
            accounts: Arc::new(RwLock::new(Vec::new())),
            http_client: reqwest::Client::new(),
            stats: Arc::new(RwLock::new(GeneratorStats::default())),
        }
    }

    /// Initialize accounts with initial balances
    pub async fn initialize_accounts(&self) -> Result<()> {
        log::info!("Initializing {} accounts with {} tokens each", 
                  self.config.num_accounts, self.config.initial_balance);
        
        let mut accounts = self.accounts.write().await;
        accounts.clear();
        
        for i in 0..self.config.num_accounts {
            let address = format!("robot_account_{:06}", i);
            accounts.push(AccountState {
                address: address.clone(),
                balance: self.config.initial_balance,
                nonce: 0,
            });
        }
        
        log::info!("Initialized {} accounts", accounts.len());
        Ok(())
    }

    /// Generate a random transaction
    async fn generate_transaction(&self) -> Result<Option<Transaction>> {
        let accounts = self.accounts.read().await;
        if accounts.len() < 2 {
            return Ok(None);
        }

        let mut rng = rand::thread_rng();
        
        // Select random sender and receiver
        let sender_idx = rng.gen_range(0..accounts.len());
        let receiver_idx = loop {
            let idx = rng.gen_range(0..accounts.len());
            if idx != sender_idx {
                break idx;
            }
        };

        let sender = &accounts[sender_idx];
        let receiver = &accounts[receiver_idx];

        // Generate random amount and fee
        let amount = rng.gen_range(self.config.amount_range.0..=self.config.amount_range.1);
        let fee = rng.gen_range(self.config.fee_range.0..=self.config.fee_range.1);

        // Check if sender has enough balance
        if sender.balance < amount + fee {
            return Ok(None);
        }

        // Create transaction
        let transaction = create_transaction(
            TransactionType::Payment {
                from: sender.address.clone(),
                to: receiver.address.clone(),
                amount,
                fee,
            },
            sender.nonce,
            sender.address.clone(),
            format!("robot_signature_{}", rng.gen::<u64>()),
        )?;

        Ok(Some(transaction))
    }

    /// Send transaction to a target node
    async fn send_transaction(&self, transaction: &Transaction, target_url: &str) -> Result<bool> {
        let tx_json = serde_json::to_value(transaction)
            .map_err(|e| IppanError::Serialization(format!("Failed to serialize transaction: {}", e)))?;

        let response = self.http_client
            .post(&format!("{}/api/v1/transaction", target_url))
            .json(&tx_json)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| IppanError::Network(format!("Failed to send transaction: {}", e)))?;

        if response.status().is_success() {
            Ok(true)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::warn!("Transaction failed with status {}: {}", status, body);
            Ok(false)
        }
    }

    /// Update account state after successful transaction
    async fn update_account_state(&self, transaction: &Transaction) -> Result<()> {
        if let TransactionType::Payment { from, to, amount, fee } = &transaction.tx_type {
            let mut accounts = self.accounts.write().await;
            
            // Update sender
            if let Some(sender) = accounts.iter_mut().find(|acc| acc.address == *from) {
                sender.balance = sender.balance.saturating_sub(amount + fee);
                sender.nonce += 1;
            }
            
            // Update receiver
            if let Some(receiver) = accounts.iter_mut().find(|acc| acc.address == *to) {
                receiver.balance += amount;
            }
        }
        Ok(())
    }

    /// Run the transaction generator
    pub async fn run(&self) -> Result<()> {
        log::info!("Starting transaction generator with config: {:?}", self.config);
        
        // Initialize accounts
        self.initialize_accounts().await?;
        
        // Set start time
        {
            let mut stats = self.stats.write().await;
            stats.start_time = Some(Instant::now());
        }
        
        let start_time = Instant::now();
        let duration = Duration::from_secs(self.config.duration_seconds);
        let interval = Duration::from_millis(1000 / self.config.tx_per_second as u64);
        
        log::info!("Generating {} transactions per second for {} seconds", 
                  self.config.tx_per_second, self.config.duration_seconds);
        
        let mut last_stats_time = Instant::now();
        
        while start_time.elapsed() < duration {
            let loop_start = Instant::now();
            
            // Generate and send transactions
            for _ in 0..self.config.tx_per_second {
                if let Some(transaction) = self.generate_transaction().await? {
                    // Select random target node
                    let mut rng = rand::thread_rng();
                    let target_url = &self.config.target_nodes[rng.gen_range(0..self.config.target_nodes.len())];
                    
                    // Send transaction
                    let success = self.send_transaction(&transaction, target_url).await?;
                    
                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        stats.total_sent += 1;
                        if success {
                            stats.total_successful += 1;
                            // Update account state
                            drop(stats);
                            self.update_account_state(&transaction).await?;
                        } else {
                            stats.total_failed += 1;
                        }
                    }
                }
            }
            
            // Log statistics every 10 seconds
            if last_stats_time.elapsed() >= Duration::from_secs(10) {
                self.log_stats().await;
                last_stats_time = Instant::now();
            }
            
            // Sleep to maintain rate
            let elapsed = loop_start.elapsed();
            if elapsed < interval {
                sleep(interval - elapsed).await;
            }
        }
        
        // Set end time
        {
            let mut stats = self.stats.write().await;
            stats.end_time = Some(Instant::now());
        }
        
        log::info!("Transaction generator completed");
        self.log_final_stats().await;
        
        Ok(())
    }

    /// Log current statistics
    async fn log_stats(&self) {
        let stats = self.stats.read().await;
        let accounts = self.accounts.read().await;
        
        let total_balance: u64 = accounts.iter().map(|acc| acc.balance).sum();
        let avg_balance = if accounts.is_empty() { 0 } else { total_balance / accounts.len() as u64 };
        
        log::info!("Generator Stats - Sent: {}, Successful: {}, Failed: {}, Avg Balance: {}", 
                  stats.total_sent, stats.total_successful, stats.total_failed, avg_balance);
    }

    /// Log final statistics
    async fn log_final_stats(&self) {
        let stats = self.stats.read().await;
        let accounts = self.accounts.read().await;
        
        let total_balance: u64 = accounts.iter().map(|acc| acc.balance).sum();
        let avg_balance = if accounts.is_empty() { 0 } else { total_balance / accounts.len() as u64 };
        
        let duration = if let (Some(start), Some(end)) = (stats.start_time, stats.end_time) {
            end.duration_since(start)
        } else {
            Duration::from_secs(0)
        };
        
        let tx_per_second = if duration.as_secs() > 0 {
            stats.total_sent as f64 / duration.as_secs() as f64
        } else {
            0.0
        };
        
        log::info!("=== FINAL GENERATOR STATISTICS ===");
        log::info!("Duration: {:?}", duration);
        log::info!("Total Transactions Sent: {}", stats.total_sent);
        log::info!("Successful Transactions: {}", stats.total_successful);
        log::info!("Failed Transactions: {}", stats.total_failed);
        log::info!("Success Rate: {:.2}%", 
                  if stats.total_sent > 0 { 
                      (stats.total_successful as f64 / stats.total_sent as f64) * 100.0 
                  } else { 
                      0.0 
                  });
        log::info!("Average TPS: {:.2}", tx_per_second);
        log::info!("Total Balance: {}", total_balance);
        log::info!("Average Account Balance: {}", avg_balance);
        log::info!("================================");
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> GeneratorStats {
        self.stats.read().await.clone()
    }
}

/// CLI tool for running the transaction generator
pub async fn run_transaction_generator() -> Result<()> {
    let config = GeneratorConfig {
        tx_per_second: 50, // 50 transactions per second
        num_accounts: 1000, // 1000 robot accounts
        initial_balance: 10000000, // 10M tokens per account
        amount_range: (1000, 100000), // 1K to 100K tokens per transaction
        fee_range: (10, 1000), // 10 to 1K tokens fee
        duration_seconds: 600, // 10 minutes
        target_nodes: vec![
            "http://188.245.97.41:3000".to_string(),
            "http://135.181.145.174:3001".to_string(),
        ],
    };

    let generator = TransactionGenerator::new(config);
    generator.run().await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_generator_creation() {
        let config = GeneratorConfig::default();
        let generator = TransactionGenerator::new(config);
        
        // Test initialization
        generator.initialize_accounts().await.unwrap();
        
        let accounts = generator.accounts.read().await;
        assert_eq!(accounts.len(), 100); // Default num_accounts
        
        for account in accounts.iter() {
            assert_eq!(account.balance, 1000000); // Default initial_balance
            assert_eq!(account.nonce, 0);
        }
    }

    #[tokio::test]
    async fn test_transaction_generation() {
        let config = GeneratorConfig {
            num_accounts: 10,
            initial_balance: 100000,
            amount_range: (100, 1000),
            fee_range: (1, 10),
            ..Default::default()
        };
        
        let generator = TransactionGenerator::new(config);
        generator.initialize_accounts().await.unwrap();
        
        // Generate a few transactions
        for _ in 0..10 {
            let transaction = generator.generate_transaction().await.unwrap();
            if let Some(tx) = transaction {
                assert!(matches!(tx.tx_type, TransactionType::Payment { .. }));
            }
        }
    }
}
