//! Transaction verifier for zk-STARK roundchain
//!
//! Verifies tx via:
//! - Merkle inclusion
//! - zk-STARK reference
//! - HashTimer signature + round metadata
//! Exposes endpoint: GET /verify_tx/:hash → { included: true, round: R, timestamp: T }

use crate::Result;
use super::{RoundHeader, ZkStarkProof, MerkleTree, TransactionVerification, RoundAggregation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Transaction verifier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxVerifierConfig {
    /// Enable transaction verification
    pub enable_verification: bool,
    /// Maximum verification time in milliseconds
    pub max_verification_time_ms: u64,
    /// Enable Merkle inclusion proof verification
    pub enable_merkle_verification: bool,
    /// Enable zk-STARK proof verification
    pub enable_zk_verification: bool,
    /// Enable HashTimer verification
    pub enable_hashtimer_verification: bool,
    /// Cache verification results
    pub enable_verification_cache: bool,
    /// Cache size limit
    pub cache_size_limit: usize,
}

impl Default for TxVerifierConfig {
    fn default() -> Self {
        Self {
            enable_verification: true,
            max_verification_time_ms: 100, // 100ms for fast verification
            enable_merkle_verification: true,
            enable_zk_verification: true,
            enable_hashtimer_verification: true,
            enable_verification_cache: true,
            cache_size_limit: 10_000,
        }
    }
}

/// Verification cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCacheEntry {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Verification result
    pub verification: TransactionVerification,
    /// Timestamp when cached
    pub cached_at: u64,
    /// Round number
    pub round_number: u64,
}

/// Transaction verifier for zk-STARK rounds
#[derive(Debug)]
pub struct TxVerifier {
    /// Configuration
    config: TxVerifierConfig,
    /// Verification cache
    verification_cache: Arc<RwLock<HashMap<[u8; 32], VerificationCacheEntry>>>,
    /// Round aggregations by round number
    round_aggregations: Arc<RwLock<HashMap<u64, RoundAggregation>>>,
    /// Verification statistics
    verification_stats: Arc<RwLock<HashMap<[u8; 32], VerificationStats>>>,
}

/// Verification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStats {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Verification time in milliseconds
    pub verification_time_ms: u64,
    /// Round number
    pub round_number: u64,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Verification method used
    pub verification_method: VerificationMethod,
}

/// Verification methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// Merkle inclusion proof
    MerkleInclusion,
    /// zk-STARK proof verification
    ZkStarkProof,
    /// HashTimer verification
    HashTimer,
    /// Cached result
    Cached,
    /// Combined verification
    Combined,
}

impl TxVerifier {
    /// Create a new transaction verifier
    pub fn new(config: TxVerifierConfig) -> Self {
        Self {
            config,
            verification_cache: Arc::new(RwLock::new(HashMap::new())),
            round_aggregations: Arc::new(RwLock::new(HashMap::new())),
            verification_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add round aggregation for verification
    pub async fn add_round_aggregation(&self, aggregation: RoundAggregation) {
        let round_number = aggregation.header.round_number;
        let mut aggregations = self.round_aggregations.write().await;
        aggregations.insert(round_number, aggregation);
        
        debug!("Added round aggregation {} for verification", round_number);
    }

    /// Verify a transaction
    pub async fn verify_transaction(&self, tx_hash: [u8; 32]) -> Result<TransactionVerification> {
        if !self.config.enable_verification {
            return Ok(TransactionVerification {
                included: false,
                round: None,
                timestamp: None,
                merkle_proof: None,
                zk_proof_reference: None,
            });
        }

        let start_time = std::time::Instant::now();

        // Check cache first
        if self.config.enable_verification_cache {
            if let Some(cached_result) = self.get_cached_verification(tx_hash).await {
                debug!("Using cached verification for transaction: {:?}", tx_hash);
                return Ok(cached_result);
            }
        }

        // Find the round containing this transaction
        let round_aggregation = self.find_transaction_round(tx_hash).await;
        
        let verification = match round_aggregation {
            Some(aggregation) => {
                self.verify_transaction_in_round(tx_hash, &aggregation).await?
            }
            None => {
                TransactionVerification {
                    included: false,
                    round: None,
                    timestamp: None,
                    merkle_proof: None,
                    zk_proof_reference: None,
                }
            }
        };

        let verification_time = start_time.elapsed().as_millis() as u64;

        // Record statistics
        self.record_verification_stats(
            tx_hash,
            verification_time,
            verification.included,
            None,
            VerificationMethod::Combined,
        ).await;

        // Cache result if enabled
        if self.config.enable_verification_cache && verification.included {
            self.cache_verification_result(tx_hash, &verification).await;
        }

        debug!(
            "Transaction verification completed: {} ms, included: {}",
            verification_time,
            verification.included
        );

        Ok(verification)
    }

    /// Find which round contains a transaction
    async fn find_transaction_round(&self, tx_hash: [u8; 32]) -> Option<RoundAggregation> {
        let aggregations = self.round_aggregations.read().await;
        
        for aggregation in aggregations.values() {
            if aggregation.transaction_hashes.contains(&tx_hash) {
                return Some(aggregation.clone());
            }
        }
        
        None
    }

    /// Verify transaction in a specific round
    async fn verify_transaction_in_round(
        &self,
        tx_hash: [u8; 32],
        aggregation: &RoundAggregation,
    ) -> Result<TransactionVerification> {
        let round_number = aggregation.header.round_number;
        
        // Find transaction index in the round
        let tx_index = aggregation.transaction_hashes.iter()
            .position(|&hash| hash == tx_hash)
            .ok_or_else(|| crate::IppanError::Validation(
                "Transaction not found in round".to_string()
            ))?;

        // Generate Merkle inclusion proof
        let merkle_proof = if self.config.enable_merkle_verification {
            aggregation.merkle_tree.generate_inclusion_proof(tx_index)
        } else {
            None
        };

        // Verify Merkle inclusion proof
        let merkle_valid = if let Some(proof) = &merkle_proof {
            aggregation.merkle_tree.verify_inclusion_proof(tx_hash, proof, tx_index)
        } else {
            true // Skip verification if disabled
        };

        if !merkle_valid {
            return Err(crate::IppanError::Validation(
                "Merkle inclusion proof verification failed".to_string()
            ));
        }

        // Verify zk-STARK proof if enabled
        if self.config.enable_zk_verification {
            self.verify_zk_proof(&aggregation.zk_proof, &aggregation.header).await?;
        }

        // Verify HashTimer if enabled
        if self.config.enable_hashtimer_verification {
            self.verify_hashtimer(&aggregation.header).await?;
        }

        // Create verification result
        let verification = TransactionVerification {
            included: true,
            round: Some(round_number),
            timestamp: Some(aggregation.header.hashtimer_timestamp),
            merkle_proof,
            zk_proof_reference: Some([0u8; 32]), // TODO: Use actual proof hash
        };

        Ok(verification)
    }

    /// Verify zk-STARK proof
    async fn verify_zk_proof(&self, proof: &ZkStarkProof, header: &RoundHeader) -> Result<()> {
        // TODO: Implement actual zk-STARK verification
        // For now, perform basic validation
        
        if proof.round_number != header.round_number {
            return Err(crate::IppanError::Validation(
                "zk-STARK proof round number mismatch".to_string()
            ));
        }

        if proof.proof_data.is_empty() {
            return Err(crate::IppanError::Validation(
                "zk-STARK proof data is empty".to_string()
            ));
        }

        debug!("zk-STARK proof verification passed for round {}", proof.round_number);
        Ok(())
    }

    /// Verify HashTimer
    async fn verify_hashtimer(&self, header: &RoundHeader) -> Result<()> {
        // TODO: Implement HashTimer verification
        // For now, perform basic validation
        
        if header.hashtimer_timestamp == 0 {
            return Err(crate::IppanError::Validation(
                "Invalid HashTimer timestamp".to_string()
            ));
        }

        debug!("HashTimer verification passed for round {}", header.round_number);
        Ok(())
    }

    /// Get cached verification result
    async fn get_cached_verification(&self, tx_hash: [u8; 32]) -> Option<TransactionVerification> {
        let cache = self.verification_cache.read().await;
        cache.get(&tx_hash).map(|entry| entry.verification.clone())
    }

    /// Cache verification result
    async fn cache_verification_result(
        &self,
        tx_hash: [u8; 32],
        verification: &TransactionVerification,
    ) {
        let mut cache = self.verification_cache.write().await;
        
        // Check cache size limit
        if cache.len() >= self.config.cache_size_limit {
            // Remove oldest entry (simple FIFO)
            let oldest_key = cache.keys().next().cloned();
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }

        let entry = VerificationCacheEntry {
            tx_hash,
            verification: verification.clone(),
            cached_at: Self::get_current_timestamp(),
            round_number: verification.round.unwrap_or(0),
        };

        cache.insert(tx_hash, entry);
        debug!("Cached verification result for transaction: {:?}", tx_hash);
    }

    /// Record verification statistics
    async fn record_verification_stats(
        &self,
        tx_hash: [u8; 32],
        verification_time: u64,
        success: bool,
        error_message: Option<String>,
        method: VerificationMethod,
    ) {
        let stats = VerificationStats {
            tx_hash,
            verification_time_ms: verification_time,
            round_number: 0, // Will be set if found
            success,
            error_message,
            verification_method: method,
        };

        let mut verification_stats = self.verification_stats.write().await;
        verification_stats.insert(tx_hash, stats);
    }

    /// Get verification statistics for a transaction
    pub async fn get_verification_stats(&self, tx_hash: [u8; 32]) -> Option<VerificationStats> {
        let stats = self.verification_stats.read().await;
        stats.get(&tx_hash).cloned()
    }

    /// Get all verification statistics
    pub async fn get_all_verification_stats(&self) -> Vec<VerificationStats> {
        let stats = self.verification_stats.read().await;
        stats.values().cloned().collect()
    }

    /// Clear verification cache
    pub async fn clear_cache(&self) {
        let mut cache = self.verification_cache.write().await;
        cache.clear();
        debug!("Cleared verification cache");
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.verification_cache.read().await;
        (cache.len(), self.config.cache_size_limit)
    }

    /// Verify multiple transactions in batch
    pub async fn verify_transactions_batch(
        &self,
        tx_hashes: Vec<[u8; 32]>,
    ) -> Result<Vec<TransactionVerification>> {
        let mut results = Vec::new();
        
        for tx_hash in tx_hashes {
            match self.verify_transaction(tx_hash).await {
                Ok(verification) => results.push(verification),
                Err(e) => {
                    warn!("Failed to verify transaction {:?}: {}", tx_hash, e);
                    results.push(TransactionVerification {
                        included: false,
                        round: None,
                        timestamp: None,
                        merkle_proof: None,
                        zk_proof_reference: None,
                    });
                }
            }
        }
        
        Ok(results)
    }

    /// Get current timestamp
    fn get_current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// HTTP endpoint handler for transaction verification
    pub async fn handle_verify_tx_endpoint(&self, tx_hash_hex: &str) -> Result<serde_json::Value> {
        // Parse transaction hash from hex
        let tx_hash = hex::decode(tx_hash_hex)
            .map_err(|_| crate::IppanError::Validation("Invalid transaction hash format".to_string()))?;
        
        if tx_hash.len() != 32 {
            return Err(crate::IppanError::Validation("Invalid transaction hash length".to_string()));
        }
        
        let mut tx_hash_array = [0u8; 32];
        tx_hash_array.copy_from_slice(&tx_hash);
        
        // Verify the transaction
        let verification = self.verify_transaction(tx_hash_array).await?;
        
        // Convert to JSON response
        let response = serde_json::json!({
            "included": verification.included,
            "round": verification.round,
            "timestamp": verification.timestamp,
            "merkle_proof": verification.merkle_proof.map(|proof| {
                proof.iter().map(|hash| hex::encode(hash)).collect::<Vec<_>>()
            }),
            "zk_proof_reference": verification.zk_proof_reference.map(|proof| {
                hex::encode(&proof[..std::cmp::min(proof.len(), 64)])
            }),
        });
        
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{blockdag::Transaction, hashtimer::HashTimer};

    #[tokio::test]
    async fn test_verifier_creation() {
        let config = TxVerifierConfig::default();
        let verifier = TxVerifier::new(config);
        
        assert!(verifier.config.enable_verification);
        assert_eq!(verifier.config.max_verification_time_ms, 100);
    }

    #[tokio::test]
    async fn test_transaction_verification() {
        let config = TxVerifierConfig::default();
        let verifier = TxVerifier::new(config);
        
        // Create test round aggregation
        let header = RoundHeader::new(1, [1u8; 32], [2u8; 32], 1234567890, [3u8; 32]);
        let hashtimer = HashTimer::new([0u8; 32], [1u8; 32]);
        let transaction = Transaction::new(
            [1u8; 32],
            [2u8; 32],
            100,
            1,
            1,
            hashtimer,
        );
        
        let aggregation = RoundAggregation {
            header,
            zk_proof: ZkStarkProof {
                proof_data: vec![1, 2, 3, 4],
                proof_size: 4,
                proving_time_ms: 100,
                verification_time_ms: 10,
                round_number: 1,
                transaction_count: 1,
            },
            transaction_hashes: vec![transaction.hash],
            merkle_tree: MerkleTree::new(vec![transaction.hash]),
        };
        
        verifier.add_round_aggregation(aggregation).await;
        
        // Verify the transaction
        let verification = verifier.verify_transaction(transaction.hash).await.unwrap();
        assert!(verification.included);
        assert_eq!(verification.round, Some(1));
    }

    #[tokio::test]
    async fn test_http_endpoint() {
        let config = TxVerifierConfig::default();
        let verifier = TxVerifier::new(config);
        
        // Test with invalid hash
        let result = verifier.handle_verify_tx_endpoint("invalid").await;
        assert!(result.is_err());
        
        // Test with valid hash
        let tx_hash_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verifier.handle_verify_tx_endpoint(tx_hash_hex).await.unwrap();
        assert_eq!(result["included"], false);
    }
} 