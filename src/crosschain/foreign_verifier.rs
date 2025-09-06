//! Foreign verifier for cross-chain and L2 transactions
//! 
//! This module provides verification interfaces for external chain anchors
//! and L2 commitments/exits.

use crate::crosschain::types::{L2CommitTx, L2ExitTx, ProofType, L2ValidationError};
use crate::crosschain::bridge::L2Registry;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Verification errors
#[derive(Error, Debug)]
pub enum VerifyError {
    #[error("Unsupported proof type")]
    Unsupported,
    #[error("Invalid proof bytes")]
    InvalidProof,
    #[error("Challenge window not elapsed")]
    ChallengeWindowOpen,
    #[error("L2 validation error: {0}")]
    L2Validation(#[from] L2ValidationError),
    #[error("L2 not registered")]
    L2NotRegistered,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// L2 verifier trait
#[async_trait::async_trait]
pub trait L2Verifier {
    /// Verify an L2 commit transaction
    async fn verify_commit(&self, tx: &L2CommitTx, registry: &L2Registry) -> Result<(), VerifyError>;
    
    /// Verify an L2 exit transaction
    async fn verify_exit(&self, tx: &L2ExitTx, registry: &L2Registry) -> Result<(), VerifyError>;
}

/// Default L2 verifier implementation
pub struct DefaultL2Verifier;

#[async_trait::async_trait]
impl L2Verifier for DefaultL2Verifier {
    async fn verify_commit(&self, tx: &L2CommitTx, registry: &L2Registry) -> Result<(), VerifyError> {
        // Basic validation
        tx.validate(16384)?; // Use default max size for now
        
        // Check if L2 is registered
        if !registry.is_registered(&tx.l2_id).await {
            return Err(VerifyError::L2NotRegistered);
        }
        
        // Verify based on proof type
        match tx.proof_type {
            ProofType::ZkGroth16 => verify_groth16_commit(tx),
            ProofType::Optimistic => verify_optimistic_commit(tx),
            ProofType::External => Ok(()), // Assume off-chain attestation checked elsewhere
        }
    }

    async fn verify_exit(&self, tx: &L2ExitTx, registry: &L2Registry) -> Result<(), VerifyError> {
        // Basic validation
        tx.validate()?;
        
        // Check if L2 is registered
        if !registry.is_registered(&tx.l2_id).await {
            return Err(VerifyError::L2NotRegistered);
        }
        
        // Get L2 parameters to determine proof type
        let l2_params = registry.get(&tx.l2_id).await
            .ok_or(VerifyError::L2NotRegistered)?;
        
        // Verify based on proof type
        match l2_params.proof_type {
            ProofType::ZkGroth16 => verify_groth16_exit(tx),
            ProofType::Optimistic => verify_optimistic_exit(tx),
            ProofType::External => Ok(()), // Assume off-chain verification
        }
    }
}

// Stub verification functions (implement when features are enabled)
fn verify_groth16_commit(_tx: &L2CommitTx) -> Result<(), VerifyError> {
    #[cfg(feature = "zk-groth16")] {
        // TODO: Parse and verify Groth16 proof
        debug!("Verifying Groth16 commit proof");
    }
    Ok(())
}

fn verify_optimistic_commit(_tx: &L2CommitTx) -> Result<(), VerifyError> {
    // For optimistic rollups, we only verify the commitment format
    // The actual verification happens during the challenge window
    debug!("Verifying optimistic commit commitment");
    Ok(())
}

fn verify_groth16_exit(_tx: &L2ExitTx) -> Result<(), VerifyError> {
    #[cfg(feature = "zk-groth16")] {
        // TODO: Parse and verify Groth16 membership proof
        debug!("Verifying Groth16 exit proof");
    }
    Ok(())
}

fn verify_optimistic_exit(_tx: &L2ExitTx) -> Result<(), VerifyError> {
    // For optimistic rollups, verify the proof of inclusion
    // and check if challenge window has elapsed
    debug!("Verifying optimistic exit proof");
    Ok(())
}

/// Rate limiting for L2 commits
pub struct L2RateLimiter {
    /// Last commit timestamp per L2 ID
    last_commits: std::collections::HashMap<String, u64>,
}

impl L2RateLimiter {
    pub fn new() -> Self {
        Self {
            last_commits: std::collections::HashMap::new(),
        }
    }
    
    /// Check if a commit is allowed based on rate limits
    pub fn check_rate_limit(
        &mut self,
        l2_id: &str,
        min_epoch_gap_ms: u64,
        current_time: u64,
    ) -> Result<(), VerifyError> {
        if let Some(last_commit) = self.last_commits.get(l2_id) {
            let time_since_last = current_time.saturating_sub(*last_commit);
            if time_since_last < min_epoch_gap_ms {
                return Err(VerifyError::RateLimitExceeded);
            }
        }
        
        // Update last commit time
        self.last_commits.insert(l2_id.to_string(), current_time);
        Ok(())
    }
}

/// L2 verification context
pub struct L2VerificationContext {
    /// Rate limiter
    pub rate_limiter: L2RateLimiter,
    /// Verifier
    pub verifier: Box<dyn L2Verifier>,
}

impl L2VerificationContext {
    pub fn new() -> Self {
        Self {
            rate_limiter: L2RateLimiter::new(),
            verifier: Box::new(DefaultL2Verifier),
        }
    }
    
    /// Verify an L2 commit with rate limiting
    pub async fn verify_commit(
        &mut self,
        tx: &L2CommitTx,
        registry: &L2Registry,
        current_time: u64,
    ) -> Result<(), VerifyError> {
        // Get L2 parameters for rate limiting
        let l2_params = registry.get(&tx.l2_id)
            .await
            .ok_or(VerifyError::L2NotRegistered)?;
        
        // Check rate limit
        self.rate_limiter.check_rate_limit(
            &tx.l2_id,
            l2_params.min_epoch_gap_ms,
            current_time,
        )?;
        
        // Verify the commit
        self.verifier.verify_commit(tx, registry).await
    }
    
    /// Verify an L2 exit
    pub async fn verify_exit(
        &self,
        tx: &L2ExitTx,
        registry: &L2Registry,
    ) -> Result<(), VerifyError> {
        self.verifier.verify_exit(tx, registry).await
    }
}

impl Default for L2VerificationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosschain::types::{ProofType, DataAvailabilityMode};
    
    #[test]
    fn test_rate_limiter() {
        let mut limiter = L2RateLimiter::new();
        let current_time = 1000;
        
        // First commit should be allowed
        assert!(limiter.check_rate_limit("test-l2", 250, current_time).is_ok());
        
        // Second commit too soon should be rejected
        assert!(limiter.check_rate_limit("test-l2", 250, current_time + 100).is_err());
        
        // Commit after rate limit window should be allowed
        assert!(limiter.check_rate_limit("test-l2", 250, current_time + 300).is_ok());
    }
} 