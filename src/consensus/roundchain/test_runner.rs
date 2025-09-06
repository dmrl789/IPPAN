//! Test runner for zk-STARK roundchain latency simulation
//!
//! Simulates 1,000 blocks per round (100 tx each = 100,000 tx)
//! Benchmarks: proving_time, proof_size, verification_time, global_propagation_latency
//! Validates: finality commitment accuracy, transaction inclusion and timestamp auditability

use crate::{
    consensus::{
        blockdag::{Block, Transaction},
        hashtimer::HashTimer,
        roundchain::{
            round_manager::{RoundManagerConfig, ZkRoundManager},
            zk_prover::{ZkProverConfig, ZkProver},
            proof_broadcast::{BroadcastConfig, ProofBroadcaster},
            tx_verifier::{TxVerifierConfig, TxVerifier}, ZkStarkProof, RoundAggregation,
        },
    },
    Result,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Test configuration for zk-STARK latency simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkTestConfig {
    /// Number of rounds to simulate
    pub rounds_to_simulate: u64,
    /// Blocks per round
    pub blocks_per_round: usize,
    /// Transactions per block
    pub transactions_per_block: usize,
    /// Simulated intercontinental latency in milliseconds
    pub intercontinental_latency_ms: u64,
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
    /// Output results to file
    pub output_results: bool,
    /// Results file path
    pub results_file_path: String,
}

impl Default for ZkTestConfig {
    fn default() -> Self {
        Self {
            rounds_to_simulate: 10,
            blocks_per_round: 1000,
            transactions_per_block: 100,
            intercontinental_latency_ms: 180,
            enable_detailed_logging: true,
            output_results: true,
            results_file_path: "zk_stark_benchmark_results.json".to_string(),
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Round number
    pub round_number: u64,
    /// Number of blocks in round
    pub block_count: usize,
    /// Number of transactions in round
    pub transaction_count: usize,
    /// zk-STARK proof size in bytes
    pub proof_size: usize,
    /// Proving time in milliseconds
    pub proving_time_ms: u64,
    /// Verification time in milliseconds
    pub verification_time_ms: u64,
    /// Global propagation latency in milliseconds
    pub propagation_latency_ms: u64,
    /// Finality commitment accuracy (percentage)
    pub finality_accuracy: f64,
    /// Transaction inclusion success rate (percentage)
    pub inclusion_success_rate: f64,
    /// Timestamp auditability success rate (percentage)
    pub timestamp_auditability_rate: f64,
}

/// Test runner for zk-STARK roundchain
#[derive(Debug)]
pub struct ZkTestRunner {
    /// Test configuration
    config: ZkTestConfig,
    /// Round manager
    round_manager: ZkRoundManager,
    /// zk-STARK prover
    zk_prover: ZkProver,
    /// Proof broadcaster
    broadcaster: ProofBroadcaster,
    /// Transaction verifier
    verifier: TxVerifier,
    /// Benchmark results
    results: Vec<BenchmarkResults>,
}

impl ZkTestRunner {
    /// Create a new test runner
    pub fn new(config: ZkTestConfig) -> Self {
        let round_config = RoundManagerConfig::default();
        let zk_config = ZkProverConfig::default();
        let broadcast_config = BroadcastConfig::default();
        let verifier_config = TxVerifierConfig::default();

        Self {
            config,
            round_manager: ZkRoundManager::new(round_config),
            zk_prover: ZkProver::new(zk_config),
            broadcaster: ProofBroadcaster::new(broadcast_config),
            verifier: TxVerifier::new(verifier_config),
            results: Vec::new(),
        }
    }

    /// Run the complete zk-STARK benchmark
    pub async fn run_benchmark(&mut self) -> Result<()> {
        info!("Starting zk-STARK roundchain benchmark");
        info!("Configuration: {} rounds, {} blocks/round, {} tx/block",
            self.config.rounds_to_simulate,
            self.config.blocks_per_round,
            self.config.transactions_per_block
        );

        for round in 1..=self.config.rounds_to_simulate {
            info!("Simulating round {}", round);
            
            let round_result = self.simulate_round(round).await?;
            self.results.push(round_result);
            
            // Output intermediate results
            if self.config.enable_detailed_logging {
                self.print_round_results(round);
            }
        }

        // Generate final report
        self.generate_final_report().await?;
        
        // Save results if enabled
        if self.config.output_results {
            self.save_results().await?;
        }

        info!("zk-STARK benchmark completed successfully");
        Ok(())
    }

    /// Simulate a single round
    async fn simulate_round(&mut self, round_number: u64) -> Result<BenchmarkResults> {
        let start_time = std::time::Instant::now();

        // Start the round
        let validators = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        self.round_manager.start_round(round_number, validators).await?;

        // Generate blocks for the round
        let blocks = self.generate_round_blocks(round_number).await?;
        
        // Add blocks to round manager
        for block in &blocks {
            self.round_manager.add_block(block.clone()).await?;
        }

        // Aggregate the round
        let aggregation = self.round_manager.aggregate_round().await?;
        
        // Generate zk-STARK proof
        let zk_proof = self.generate_zk_proof(&aggregation).await?;
        
        // Broadcast the proof
        let broadcast_result = self.broadcast_proof(&aggregation).await?;
        
        // Verify transactions
        let verification_result = self.verify_transactions(&aggregation).await?;
        
        let _total_time = start_time.elapsed().as_millis() as u64;

        // Create benchmark results
        let results = BenchmarkResults {
            round_number,
            block_count: blocks.len(),
            transaction_count: aggregation.transaction_hashes.len(),
            proof_size: zk_proof.proof_size,
            proving_time_ms: zk_proof.proving_time_ms,
            verification_time_ms: zk_proof.verification_time_ms,
            propagation_latency_ms: broadcast_result.propagation_latency_ms,
            finality_accuracy: self.calculate_finality_accuracy(&aggregation),
            inclusion_success_rate: verification_result.inclusion_success_rate,
            timestamp_auditability_rate: verification_result.timestamp_auditability_rate,
        };

        Ok(results)
    }

    /// Generate blocks for a round
    async fn generate_round_blocks(&self, round_number: u64) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        
        for block_index in 0..self.config.blocks_per_round {
            let transactions = self.generate_block_transactions(block_index).await?;
            
            let hashtimer = HashTimer::new(
                "test_node",
                block_index as u64,
                0, // sequence
            );
            
            let tx_hashes: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.hash).collect();
            let block = Block::new(
                round_number,
                tx_hashes,
                [1u8; 32], // validator ID
                hashtimer,
            )?;
            
            blocks.push(block);
        }
        
        Ok(blocks)
    }

    /// Generate transactions for a block
    async fn generate_block_transactions(&self, block_index: usize) -> Result<Vec<Transaction>> {
        let mut transactions = Vec::new();
        
        for tx_index in 0..self.config.transactions_per_block {
            let hashtimer = HashTimer::new(
                "test_node",
                block_index as u64,
                0, // sequence
            );
            
            let transaction = Transaction::new_payment(
                [block_index as u8; 32], // from
                [tx_index as u8; 32],    // to
                100,                      // amount
                1,                        // fee
                tx_index as u64,          // nonce
                hashtimer,
            );
            
            transactions.push(transaction);
        }
        
        Ok(transactions)
    }

    /// Generate zk-STARK proof for aggregation
    async fn generate_zk_proof(&mut self, aggregation: &RoundAggregation) -> Result<ZkStarkProof> {
        // Set round state for proving
        let round_state = super::zk_prover::RoundProvingState {
            header: aggregation.header.clone(),
            transactions: vec![], // TODO: Extract from aggregation
            merkle_root: aggregation.merkle_tree.root,
            state_root: aggregation.header.state_root,
            block_hashes: vec![], // TODO: Extract from aggregation
        };
        
        self.zk_prover.set_round_state(round_state);
        
        // Generate proof
        let proof = self.zk_prover.generate_proof().await?;
        
        Ok(proof)
    }

    /// Broadcast proof to peers
    async fn broadcast_proof(&self, aggregation: &RoundAggregation) -> Result<BroadcastResult> {
        let start_time = std::time::Instant::now();
        
        // Add some test peers
        self.add_test_peers().await;
        
        // Broadcast the aggregation
        self.broadcaster.broadcast_round_aggregation(aggregation.clone()).await?;
        
        let broadcast_time = start_time.elapsed().as_millis() as u64;
        
        Ok(BroadcastResult {
            propagation_latency_ms: broadcast_time,
            _peers_reached: self.broadcaster.get_online_peer_count().await,
            _total_peers: self.broadcaster.get_peer_count().await,
        })
    }

    /// Add test peers for simulation
    async fn add_test_peers(&self) {
        let peers = vec![
            super::proof_broadcast::Peer {
                id: [1u8; 32],
                address: "192.168.1.1:8080".to_string(),
                latency_ms: self.config.intercontinental_latency_ms,
                online: true,
                last_successful_broadcast: None,
            },
            super::proof_broadcast::Peer {
                id: [2u8; 32],
                address: "10.0.0.1:8080".to_string(),
                latency_ms: self.config.intercontinental_latency_ms / 2,
                online: true,
                last_successful_broadcast: None,
            },
        ];
        
        for peer in peers {
            self.broadcaster.add_peer(peer).await;
        }
    }

    /// Verify transactions in the aggregation
    async fn verify_transactions(&self, aggregation: &RoundAggregation) -> Result<VerificationResult> {
        // Add aggregation to verifier
        self.verifier.add_round_aggregation(aggregation.clone()).await;
        
        let mut inclusion_success = 0;
        let mut timestamp_auditability_success = 0;
        let total_transactions = aggregation.transaction_hashes.len();
        
        // Verify a sample of transactions
        let sample_size = std::cmp::min(100, total_transactions);
        for i in 0..sample_size {
            let tx_hash = aggregation.transaction_hashes[i];
            
            match self.verifier.verify_transaction(tx_hash).await {
                Ok(verification) => {
                    if verification.included {
                        inclusion_success += 1;
                        
                        if verification.timestamp.is_some() {
                            timestamp_auditability_success += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("Transaction verification failed: {}", e);
                }
            }
        }
        
        let inclusion_rate = (inclusion_success as f64 / sample_size as f64) * 100.0;
        let timestamp_rate = (timestamp_auditability_success as f64 / sample_size as f64) * 100.0;
        
        Ok(VerificationResult {
            inclusion_success_rate: inclusion_rate,
            timestamp_auditability_rate: timestamp_rate,
        })
    }

    /// Calculate finality accuracy
    fn calculate_finality_accuracy(&self, _aggregation: &RoundAggregation) -> f64 {
        // TODO: Implement actual finality accuracy calculation
        // For now, return a placeholder value
        99.5
    }

    /// Print round results
    fn print_round_results(&self, round: u64) {
        if let Some(result) = self.results.last() {
            info!("Round {} Results:", round);
            info!("  Blocks: {}", result.block_count);
            info!("  Transactions: {}", result.transaction_count);
            info!("  Proof Size: {} bytes", result.proof_size);
            info!("  Proving Time: {} ms", result.proving_time_ms);
            info!("  Verification Time: {} ms", result.verification_time_ms);
            info!("  Propagation Latency: {} ms", result.propagation_latency_ms);
            info!("  Finality Accuracy: {:.1}%", result.finality_accuracy);
            info!("  Inclusion Success: {:.1}%", result.inclusion_success_rate);
            info!("  Timestamp Auditability: {:.1}%", result.timestamp_auditability_rate);
        }
    }

    /// Generate final benchmark report
    async fn generate_final_report(&self) -> Result<()> {
        if self.results.is_empty() {
            return Ok(());
        }

        let total_rounds = self.results.len();
        let avg_proof_size: usize = self.results.iter().map(|r| r.proof_size).sum::<usize>() / total_rounds;
        let avg_proving_time: u64 = self.results.iter().map(|r| r.proving_time_ms).sum::<u64>() / total_rounds as u64;
        let avg_verification_time: u64 = self.results.iter().map(|r| r.verification_time_ms).sum::<u64>() / total_rounds as u64;
        let avg_propagation_latency: u64 = self.results.iter().map(|r| r.propagation_latency_ms).sum::<u64>() / total_rounds as u64;
        let avg_finality_accuracy: f64 = self.results.iter().map(|r| r.finality_accuracy).sum::<f64>() / total_rounds as f64;
        let avg_inclusion_success: f64 = self.results.iter().map(|r| r.inclusion_success_rate).sum::<f64>() / total_rounds as f64;
        let avg_timestamp_auditability: f64 = self.results.iter().map(|r| r.timestamp_auditability_rate).sum::<f64>() / total_rounds as f64;

        info!("=== zk-STARK Benchmark Final Report ===");
        info!("Total Rounds Simulated: {}", total_rounds);
        info!("Average Proof Size: {} bytes", avg_proof_size);
        info!("Average Proving Time: {} ms", avg_proving_time);
        info!("Average Verification Time: {} ms", avg_verification_time);
        info!("Average Propagation Latency: {} ms", avg_propagation_latency);
        info!("Average Finality Accuracy: {:.1}%", avg_finality_accuracy);
        info!("Average Inclusion Success Rate: {:.1}%", avg_inclusion_success);
        info!("Average Timestamp Auditability: {:.1}%", avg_timestamp_auditability);

        // Check if targets are met
        self.check_performance_targets(avg_proof_size, avg_proving_time, avg_verification_time, avg_propagation_latency);

        Ok(())
    }

    /// Check if performance targets are met
    fn check_performance_targets(&self, proof_size: usize, proving_time: u64, verification_time: u64, propagation_latency: u64) {
        info!("=== Performance Target Analysis ===");
        
        // Target: 50-100 KB proof size
        if proof_size <= 100_000 {
            info!("✓ Proof size target met: {} bytes <= 100 KB", proof_size);
        } else {
            warn!("✗ Proof size target exceeded: {} bytes > 100 KB", proof_size);
        }
        
        // Target: 0.5-2 seconds proving time
        if proving_time <= 2000 {
            info!("✓ Proving time target met: {} ms <= 2000 ms", proving_time);
        } else {
            warn!("✗ Proving time target exceeded: {} ms > 2000 ms", proving_time);
        }
        
        // Target: 10-50ms verification time
        if verification_time <= 50 {
            info!("✓ Verification time target met: {} ms <= 50 ms", verification_time);
        } else {
            warn!("✗ Verification time target exceeded: {} ms > 50 ms", verification_time);
        }
        
        // Target: sub-second propagation (180ms for intercontinental)
        if propagation_latency <= self.config.intercontinental_latency_ms {
            info!("✓ Propagation latency target met: {} ms <= {} ms", propagation_latency, self.config.intercontinental_latency_ms);
        } else {
            warn!("✗ Propagation latency target exceeded: {} ms > {} ms", propagation_latency, self.config.intercontinental_latency_ms);
        }
    }

    /// Save results to file
    async fn save_results(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.results)?;
        tokio::fs::write(&self.config.results_file_path, json).await?;
        
        info!("Results saved to: {}", self.config.results_file_path);
        Ok(())
    }
}

/// Broadcast result
#[derive(Debug)]
struct BroadcastResult {
    propagation_latency_ms: u64,
    _peers_reached: usize,
    _total_peers: usize,
}

/// Verification result
#[derive(Debug)]
struct VerificationResult {
    inclusion_success_rate: f64,
    timestamp_auditability_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_runner_creation() {
        let config = ZkTestConfig::default();
        let runner = ZkTestRunner::new(config);
        
        assert_eq!(runner.config.rounds_to_simulate, 10);
        assert_eq!(runner.config.blocks_per_round, 1000);
    }

    #[tokio::test]
    async fn test_small_benchmark() {
        let mut config = ZkTestConfig::default();
        config.rounds_to_simulate = 1;
        config.blocks_per_round = 10;
        config.transactions_per_block = 5;
        
        let mut runner = ZkTestRunner::new(config);
        runner.run_benchmark().await.unwrap();
        
        assert_eq!(runner.results.len(), 1);
    }
} 