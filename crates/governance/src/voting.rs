use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vote on a governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Voter address
    pub voter: [u8; 32],
    /// Proposal ID being voted on
    pub proposal_id: String,
    /// Vote choice (true = approve, false = reject)
    pub approve: bool,
    /// Voter's stake weight
    pub stake_weight: u64,
    /// Vote timestamp
    pub timestamp: u64,
    /// Vote signature
    pub signature: [u8; 64],
}

/// Voting power calculation
pub struct VotingPowerCalculator {
    /// Total stake in the system
    total_stake: u64,
    /// Stake per validator
    validator_stakes: HashMap<[u8; 32], u64>,
}

impl VotingPowerCalculator {
    /// Create a new voting power calculator
    pub fn new() -> Self {
        Self {
            total_stake: 0,
            validator_stakes: HashMap::new(),
        }
    }

    /// Add or update a validator's stake
    pub fn update_stake(&mut self, validator: [u8; 32], stake: u64) {
        if let Some(old_stake) = self.validator_stakes.insert(validator, stake) {
            self.total_stake = self.total_stake.saturating_sub(old_stake);
        }
        self.total_stake = self.total_stake.saturating_add(stake);
    }

    /// Remove a validator's stake
    pub fn remove_stake(&mut self, validator: &[u8; 32]) {
        if let Some(stake) = self.validator_stakes.remove(validator) {
            self.total_stake = self.total_stake.saturating_sub(stake);
        }
    }

    /// Get voting power for a validator
    pub fn get_voting_power(&self, validator: &[u8; 32]) -> u64 {
        self.validator_stakes.get(validator).copied().unwrap_or(0)
    }

    /// Get total stake
    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }

    /// Calculate voting power percentage
    pub fn get_voting_percentage(&self, validator: &[u8; 32]) -> f64 {
        if self.total_stake == 0 {
            return 0.0;
        }
        self.get_voting_power(validator) as f64 / self.total_stake as f64
    }
}

/// Voting session for a specific proposal
#[derive(Debug, Clone)]
pub struct VotingSession {
    /// Proposal ID
    pub proposal_id: String,
    /// Voting start time
    pub start_time: u64,
    /// Voting end time
    pub end_time: u64,
    /// Votes cast
    pub votes: HashMap<[u8; 32], Vote>,
    /// Total stake that voted
    pub total_voting_stake: u64,
    /// Total stake that voted to approve
    pub approval_stake: u64,
    /// Voting threshold (percentage)
    pub threshold: f64,
}

impl VotingSession {
    /// Create a new voting session
    pub fn new(proposal_id: String, duration_seconds: u64, threshold: f64) -> Self {
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            proposal_id,
            start_time,
            end_time: start_time + duration_seconds,
            votes: HashMap::new(),
            total_voting_stake: 0,
            approval_stake: 0,
            threshold,
        }
    }

    /// Cast a vote
    pub fn cast_vote(&mut self, vote: Vote, voting_power: u64) -> Result<()> {
        // Check if voting is still open
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if current_time > self.end_time {
            return Err(anyhow::anyhow!("Voting period has ended"));
        }

        // Check if voter has already voted
        if self.votes.contains_key(&vote.voter) {
            return Err(anyhow::anyhow!("Voter has already voted"));
        }

        // Verify vote signature
        if !self.verify_vote_signature(&vote) {
            return Err(anyhow::anyhow!("Invalid vote signature"));
        }

        // Add vote
        self.votes.insert(vote.voter, vote.clone());
        self.total_voting_stake = self.total_voting_stake.saturating_add(voting_power);
        
        if vote.approve {
            self.approval_stake = self.approval_stake.saturating_add(voting_power);
        }

        Ok(())
    }

    /// Check if the proposal has passed
    pub fn has_passed(&self) -> bool {
        if self.total_voting_stake == 0 {
            return false;
        }
        
        let approval_percentage = self.approval_stake as f64 / self.total_voting_stake as f64;
        approval_percentage >= self.threshold
    }

    /// Check if voting is still open
    pub fn is_open(&self) -> bool {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        current_time <= self.end_time
    }

    /// Get voting results
    pub fn get_results(&self) -> VotingResults {
        VotingResults {
            total_votes: self.votes.len(),
            total_stake: self.total_voting_stake,
            approval_stake: self.approval_stake,
            rejection_stake: self.total_voting_stake - self.approval_stake,
            approval_percentage: if self.total_voting_stake > 0 {
                self.approval_stake as f64 / self.total_voting_stake as f64
            } else {
                0.0
            },
            threshold: self.threshold,
            passed: self.has_passed(),
        }
    }

    /// Verify vote signature
    fn verify_vote_signature(&self, vote: &Vote) -> bool {
        use ed25519_dalek::{VerifyingKey, Signature};
        
        let verifying_key = match VerifyingKey::from_bytes(&vote.voter) {
            Ok(key) => key,
            Err(_) => return false,
        };
        
        let signature = match Signature::from_bytes(&vote.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        
        // Create message for signature verification
        let mut message = Vec::new();
        message.extend_from_slice(&vote.proposal_id.as_bytes());
        message.extend_from_slice(&vote.approve.to_be_bytes());
        message.extend_from_slice(&vote.stake_weight.to_be_bytes());
        message.extend_from_slice(&vote.timestamp.to_be_bytes());
        
        verifying_key.verify(&message, &signature).is_ok()
    }
}

/// Voting results for a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingResults {
    /// Total number of votes cast
    pub total_votes: usize,
    /// Total stake that voted
    pub total_stake: u64,
    /// Stake that voted to approve
    pub approval_stake: u64,
    /// Stake that voted to reject
    pub rejection_stake: u64,
    /// Approval percentage
    pub approval_percentage: f64,
    /// Required threshold
    pub threshold: f64,
    /// Whether the proposal passed
    pub passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn create_test_vote(proposal_id: &str, approve: bool) -> Vote {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey = signing_key.verifying_key().to_bytes();
        
        let mut message = Vec::new();
        message.extend_from_slice(proposal_id.as_bytes());
        message.extend_from_slice(&approve.to_be_bytes());
        message.extend_from_slice(&1000u64.to_be_bytes());
        message.extend_from_slice(&1234567890u64.to_be_bytes());
        
        let signature = signing_key.sign(&message);
        
        Vote {
            voter: pubkey,
            proposal_id: proposal_id.to_string(),
            approve,
            stake_weight: 1000,
            timestamp: 1234567890,
            signature: signature.to_bytes(),
        }
    }

    #[test]
    fn test_voting_power_calculation() {
        let mut calculator = VotingPowerCalculator::new();
        
        calculator.update_stake([1u8; 32], 1000);
        calculator.update_stake([2u8; 32], 2000);
        
        assert_eq!(calculator.total_stake(), 3000);
        assert_eq!(calculator.get_voting_power(&[1u8; 32]), 1000);
        assert_eq!(calculator.get_voting_power(&[2u8; 32]), 2000);
        assert_eq!(calculator.get_voting_percentage(&[1u8; 32]), 1.0 / 3.0);
        assert_eq!(calculator.get_voting_percentage(&[2u8; 32]), 2.0 / 3.0);
    }

    #[test]
    fn test_voting_session() {
        let mut session = VotingSession::new("proposal_1".to_string(), 3600, 0.67);
        
        let vote1 = create_test_vote("proposal_1", true);
        let vote2 = create_test_vote("proposal_1", false);
        
        assert!(session.cast_vote(vote1, 1000).is_ok());
        assert!(session.cast_vote(vote2, 500).is_ok());
        
        let results = session.get_results();
        assert_eq!(results.total_votes, 2);
        assert_eq!(results.total_stake, 1500);
        assert_eq!(results.approval_stake, 1000);
        assert_eq!(results.rejection_stake, 500);
        assert!((results.approval_percentage - 2.0 / 3.0).abs() < 0.001);
        assert!(results.passed); // 67% threshold, we have 66.7%
    }

    #[test]
    fn test_duplicate_vote() {
        let mut session = VotingSession::new("proposal_1".to_string(), 3600, 0.67);
        
        let vote = create_test_vote("proposal_1", true);
        
        assert!(session.cast_vote(vote.clone(), 1000).is_ok());
        assert!(session.cast_vote(vote, 1000).is_err());
    }
}