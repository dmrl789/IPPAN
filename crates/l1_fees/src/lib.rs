use ippan_types::Transaction;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const BASIS_POINTS_DENOM: u16 = 10_000;
const DEFAULT_BASE_FEE: u64 = 1_000; // 0.001 mIPN (~ deterministic entry fee)
const DEFAULT_PER_BYTE_FEE: u64 = 10;
const DEFAULT_MIN_FEE: u64 = 1_000;
const DEFAULT_MAX_FEE: u64 = 10_000_000;
const DEFAULT_VALIDATOR_SHARE_BPS: u16 = 8_000; // 80%
const DEFAULT_TREASURY_SHARE_BPS: u16 = 2_000; // 20%

/// Deterministic L1 fee policy for payment transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeePolicy {
    pub base_fee: u64,
    pub per_byte_fee: u64,
    pub min_fee: u64,
    pub max_fee: u64,
    pub validator_share_bps: u16,
    pub treasury_share_bps: u16,
}

impl Default for FeePolicy {
    fn default() -> Self {
        Self {
            base_fee: DEFAULT_BASE_FEE,
            per_byte_fee: DEFAULT_PER_BYTE_FEE,
            min_fee: DEFAULT_MIN_FEE,
            max_fee: DEFAULT_MAX_FEE,
            validator_share_bps: DEFAULT_VALIDATOR_SHARE_BPS,
            treasury_share_bps: DEFAULT_TREASURY_SHARE_BPS,
        }
    }
}

/// Fee estimation details for observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeEstimate {
    pub required_fee: u64,
    pub estimated_size: usize,
}

/// Split of a collected fee between validator and treasury.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSplit {
    pub total_fee: u128,
    pub validator_fee: u128,
    pub treasury_fee: u128,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FeePolicyError {
    #[error(
        "fee split misconfigured: validator ({validator}) + treasury ({treasury}) != 10000 bps"
    )]
    InvalidSplit { validator: u16, treasury: u16 },
    #[error("fee {provided} below required minimum {required}")]
    FeeBelowMinimum { required: u64, provided: u128 },
}

impl FeePolicy {
    pub fn validate(&self) -> Result<(), FeePolicyError> {
        let total = self
            .validator_share_bps
            .saturating_add(self.treasury_share_bps);
        if total != BASIS_POINTS_DENOM {
            return Err(FeePolicyError::InvalidSplit {
                validator: self.validator_share_bps,
                treasury: self.treasury_share_bps,
            });
        }
        Ok(())
    }

    /// Deterministically estimate the fee for a transaction using the configured policy.
    pub fn estimate_fee(&self, tx: &Transaction) -> FeeEstimate {
        let size = tx_serialized_size(tx);
        // Include memo/topics metadata precisely for deterministic results.
        if tx.visibility == ippan_types::TransactionVisibility::Confidential {
            // The confidential envelope fields were already counted within tx_serialized_size.
        }

        let dynamic_fee = self.per_byte_fee.saturating_mul(size as u64);
        let mut fee = self.base_fee.saturating_add(dynamic_fee);
        fee = fee.clamp(self.min_fee, self.max_fee);

        FeeEstimate {
            required_fee: fee,
            estimated_size: size,
        }
    }

    /// Convenience helper for callers that only need the scalar fee value.
    pub fn required_fee(&self, tx: &Transaction) -> u64 {
        self.estimate_fee(tx).required_fee
    }

    /// Validate that an optional user-provided fee limit covers the required fee amount.
    pub fn enforce_fee_limit(
        &self,
        fee_limit: Option<u128>,
        required_fee: u64,
    ) -> Result<(), FeePolicyError> {
        if let Some(limit) = fee_limit {
            if limit < required_fee as u128 {
                return Err(FeePolicyError::FeeBelowMinimum {
                    required: required_fee,
                    provided: limit,
                });
            }
        }
        Ok(())
    }

    /// Compute the validator/treasury split for a collected fee amount (in atomic units).
    pub fn split_fee(&self, total_fee: u128) -> FeeSplit {
        let validator_fee = total_fee.saturating_mul(self.validator_share_bps as u128)
            / (BASIS_POINTS_DENOM as u128);
        let treasury_fee = total_fee.saturating_sub(validator_fee);
        FeeSplit {
            total_fee,
            validator_fee,
            treasury_fee,
        }
    }
}

fn tx_serialized_size(tx: &Transaction) -> usize {
    let mut size = 0usize;
    size += 32 * 3; // id, from, to
    size += 8 * 2; // amount + nonce (u64)
    size += 64; // signature bytes
    size += std::mem::size_of_val(&tx.hashtimer.timestamp_us);
    size += tx.hashtimer.entropy.len();
    size += std::mem::size_of_val(&tx.timestamp.0);

    // Topics/memo
    size += tx.topics.iter().map(|t| t.len()).sum::<usize>();

    if let Some(env) = &tx.confidential {
        size += env.enc_algo.len() + env.iv.len() + env.ciphertext.len();
        size += env
            .access_keys
            .iter()
            .map(|k| k.recipient_pub.len() + k.enc_key.len())
            .sum::<usize>();
    }

    if let Some(proof) = &tx.zk_proof {
        size += proof.proof.len();
        size += proof
            .public_inputs
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>();
    }

    if let Some(handle_op) = tx.handle_operation() {
        size += 1; // presence flag
        match handle_op {
            ippan_types::HandleOperation::Register(data) => {
                size += 1; // variant discriminator
                size += data.handle.len();
                size += data.owner.len();
                size += 1; // expiry flag
                if data.expires_at.is_some() {
                    size += std::mem::size_of::<u64>();
                }
                size += std::mem::size_of::<u32>(); // metadata len prefix
                for (key, value) in data.metadata.iter() {
                    size += std::mem::size_of::<u32>() + key.len();
                    size += std::mem::size_of::<u32>() + value.len();
                }
                size += std::mem::size_of::<u32>() + data.signature.len();
            }
        }
    } else {
        size += 1; // absence flag
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{Amount, Transaction};

    fn test_transaction(topic_size: usize) -> Transaction {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1_000), 1);
        if topic_size > 0 {
            tx.set_topics(vec!["a".repeat(topic_size)]);
        }
        tx
    }

    #[test]
    fn default_policy_estimates_fee() {
        let policy = FeePolicy::default();
        let tx = test_transaction(0);
        let estimate = policy.estimate_fee(&tx);
        assert!(estimate.required_fee >= DEFAULT_MIN_FEE);
        assert!(estimate.estimated_size > 0);
    }

    #[test]
    fn fee_limit_validation() {
        let policy = FeePolicy::default();
        let tx = test_transaction(128);
        let required = policy.required_fee(&tx);
        assert!(policy
            .enforce_fee_limit(Some(required as u128), required)
            .is_ok());
        let err = policy
            .enforce_fee_limit(Some((required - 1) as u128), required)
            .unwrap_err();
        assert_eq!(
            err,
            FeePolicyError::FeeBelowMinimum {
                required,
                provided: (required - 1) as u128
            }
        );
    }

    #[test]
    fn fee_split_matches_bps() {
        let policy = FeePolicy::default();
        let split = policy.split_fee(1_000_000);
        assert_eq!(split.total_fee, 1_000_000);
        assert_eq!(split.validator_fee, 800_000);
        assert_eq!(split.treasury_fee, 200_000);
    }
}
