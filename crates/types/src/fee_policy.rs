//! IPPAN Fee Policy Types (v1)
//!
//! Defines canonical units (kambei), fee schedules, registry pricing,
//! and epoch-based fee pool management.
//!
//! ## Units
//! - 1 IPN = 100,000,000 kambei (8 decimals, Bitcoin-style)
//! - All consensus-level fee values use kambei (u64) or u128 for intermediate math
//! - NO floating point allowed in fee computation
//!
//! ## Fee Policy Rules
//! - Fees are NEVER burned
//! - All fees go to a deterministic weekly fee pool
//! - Redistribution happens at epoch close (protocol-only spend)

use serde::{Deserialize, Serialize};

// =============================================================================
// CANONICAL UNITS
// =============================================================================

/// Number of kambei per IPN (8 decimal places, Bitcoin-style)
pub const KAMBEI_PER_IPN: u64 = 100_000_000;

/// Epoch duration in seconds (7 days)
pub const EPOCH_SECS: u64 = 7 * 24 * 60 * 60;

/// Epoch duration in microseconds
pub const EPOCH_US: u64 = EPOCH_SECS * 1_000_000;

/// Byte quantum for fee calculation (1 byte = 1 unit for simplicity)
pub const BYTE_QUANTUM: u32 = 1;

/// Epoch type (weekly epoch number since genesis)
pub type Epoch = u64;

/// Amount in kambei (smallest unit, 8 decimals)
pub type Kambei = u64;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Checked addition for kambei amounts
#[inline]
pub fn checked_add_kambei(a: Kambei, b: Kambei) -> Option<Kambei> {
    a.checked_add(b)
}

/// Checked subtraction for kambei amounts
#[inline]
pub fn checked_sub_kambei(a: Kambei, b: Kambei) -> Option<Kambei> {
    a.checked_sub(b)
}

/// Safe multiplication followed by division using u128 intermediate
/// Returns None if divisor is zero
#[inline]
pub fn mul_div_u128(n: u128, mul: u128, div: u128) -> Option<u128> {
    if div == 0 {
        return None;
    }
    n.checked_mul(mul).map(|product| product / div)
}

/// Convert IPN to kambei (saturating)
#[inline]
pub const fn ipn_to_kambei(ipn: u64) -> Kambei {
    ipn.saturating_mul(KAMBEI_PER_IPN)
}

/// Convert kambei to IPN (truncating)
#[inline]
pub const fn kambei_to_ipn(kambei: Kambei) -> u64 {
    kambei / KAMBEI_PER_IPN
}

/// Compute epoch number from IPPAN time in microseconds
/// Uses integer division - epoch 0 starts at genesis (t_us = 0)
#[inline]
pub const fn epoch_from_ippan_time_us(t_us: u64) -> Epoch {
    t_us / EPOCH_US
}

/// Compute epoch number from IPPAN time in seconds
#[inline]
pub const fn epoch_from_ippan_time_secs(t_secs: u64) -> Epoch {
    t_secs / EPOCH_SECS
}

/// Compute the start timestamp (in microseconds) of a given epoch
#[inline]
pub const fn epoch_start_us(epoch: Epoch) -> u64 {
    epoch.saturating_mul(EPOCH_US)
}

/// Compute the end timestamp (in microseconds) of a given epoch (exclusive)
#[inline]
pub const fn epoch_end_us(epoch: Epoch) -> u64 {
    epoch.saturating_add(1).saturating_mul(EPOCH_US)
}

/// Derive deterministic fee pool account ID for an epoch
/// `pool_id = BLAKE3("FEE_POOL" || epoch.to_le_bytes())`
///
/// This creates a protocol-owned address with no private key -
/// funds can only leave via the DistributeFees transition.
pub fn fee_pool_account_id(epoch: Epoch) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"FEE_POOL");
    hasher.update(&epoch.to_le_bytes());
    *hasher.finalize().as_bytes()
}

// =============================================================================
// FEE SCHEDULE V1
// =============================================================================

/// Transaction fee schedule (all values in kambei)
///
/// Fee computation:
/// `fee = max(tx_min_fee, tx_base_fee + tx_byte_fee * (tx_bytes / BYTE_QUANTUM))`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeScheduleV1 {
    /// Version of this schedule (for governance tracking)
    pub version: u32,
    /// Base fee charged for every transaction (kambei)
    pub tx_base_fee: Kambei,
    /// Fee per byte quantum (kambei)
    pub tx_byte_fee: Kambei,
    /// Minimum fee per transaction (kambei)
    pub tx_min_fee: Kambei,
    /// Maximum fee per transaction (kambei) - for DoS protection
    pub tx_max_fee: Kambei,
}

impl Default for FeeScheduleV1 {
    fn default() -> Self {
        Self {
            version: 1,
            // Base fee: 100 kambei = 0.000001 IPN
            tx_base_fee: 100,
            // Per-byte fee: 1 kambei
            tx_byte_fee: 1,
            // Minimum fee: 100 kambei
            tx_min_fee: 100,
            // Maximum fee: 10 IPN (prevents absurd fees)
            tx_max_fee: 10 * KAMBEI_PER_IPN,
        }
    }
}

impl FeeScheduleV1 {
    /// Compute the required fee for a transaction of given byte size
    ///
    /// Formula: `max(tx_min_fee, tx_base_fee + tx_byte_fee * (tx_bytes / BYTE_QUANTUM))`
    pub fn compute_tx_fee(&self, tx_bytes: u32) -> Kambei {
        let byte_component = (tx_bytes / BYTE_QUANTUM) as u64;
        let dynamic_fee = self.tx_byte_fee.saturating_mul(byte_component);
        let total = self.tx_base_fee.saturating_add(dynamic_fee);

        // Clamp to [min_fee, max_fee]
        total.clamp(self.tx_min_fee, self.tx_max_fee)
    }

    /// Validate that this schedule meets minimum floor requirements
    pub fn validate(&self) -> Result<(), FeeScheduleError> {
        // Minimum floors: at least 1 kambei for base and min fees
        if self.tx_base_fee < 1 {
            return Err(FeeScheduleError::BaseFeeBelow1Kambei);
        }
        if self.tx_min_fee < 1 {
            return Err(FeeScheduleError::MinFeeBelow1Kambei);
        }
        if self.tx_max_fee < self.tx_min_fee {
            return Err(FeeScheduleError::MaxFeeBelowMinFee);
        }
        Ok(())
    }

    /// Check if transitioning from old to new schedule satisfies rate limits (±20% per epoch)
    pub fn validate_rate_limits(&self, old: &FeeScheduleV1) -> Result<(), FeeScheduleError> {
        // Each field must be within ±20%: new <= old * 12/10 AND new >= old * 8/10
        check_rate_limit("tx_base_fee", old.tx_base_fee, self.tx_base_fee)?;
        check_rate_limit("tx_byte_fee", old.tx_byte_fee, self.tx_byte_fee)?;
        check_rate_limit("tx_min_fee", old.tx_min_fee, self.tx_min_fee)?;
        check_rate_limit("tx_max_fee", old.tx_max_fee, self.tx_max_fee)?;
        Ok(())
    }
}

/// Check that a field change is within ±20%
fn check_rate_limit(field: &str, old: u64, new: u64) -> Result<(), FeeScheduleError> {
    // new <= old * 12 / 10 (max +20%)
    let upper_bound = old.saturating_mul(12) / 10;
    // new >= old * 8 / 10 (max -20%)
    let lower_bound = old.saturating_mul(8) / 10;

    if new > upper_bound {
        return Err(FeeScheduleError::RateLimitExceeded {
            field: field.to_string(),
            old,
            new,
            max_allowed: upper_bound,
        });
    }
    if new < lower_bound {
        return Err(FeeScheduleError::RateLimitExceeded {
            field: field.to_string(),
            old,
            new,
            max_allowed: lower_bound, // Actually min_allowed in this case
        });
    }
    Ok(())
}

// =============================================================================
// REGISTRY PRICE SCHEDULE V1
// =============================================================================

/// Handle and domain registration price schedule (all values in kambei)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryPriceScheduleV1 {
    /// Version of this schedule
    pub version: u32,
    /// Base registration fee for a standard handle (kambei)
    pub handle_register_fee: Kambei,
    /// Fee for updating handle metadata (kambei)
    pub handle_update_fee: Kambei,
    /// Fee for renewing handle registration (kambei per year)
    pub handle_renew_fee_per_year: Kambei,
    /// Fee for transferring handle ownership (kambei)
    pub handle_transfer_fee: Kambei,
    /// Premium handle registration fee multiplier (basis points, 10000 = 1.0x)
    pub premium_handle_multiplier_bps: u16,
    /// Domain registration fee (kambei)
    pub domain_register_fee: Kambei,
    /// Domain renewal fee per year (kambei)
    pub domain_renew_fee_per_year: Kambei,
}

impl Default for RegistryPriceScheduleV1 {
    fn default() -> Self {
        Self {
            version: 1,
            // 0.01 IPN to register a handle
            handle_register_fee: KAMBEI_PER_IPN / 100,
            // 0.001 IPN to update metadata
            handle_update_fee: KAMBEI_PER_IPN / 1000,
            // 0.005 IPN per year to renew
            handle_renew_fee_per_year: KAMBEI_PER_IPN / 200,
            // 0.001 IPN to transfer
            handle_transfer_fee: KAMBEI_PER_IPN / 1000,
            // Premium handles cost 2x (20000 bps = 2.0x)
            premium_handle_multiplier_bps: 20000,
            // 0.1 IPN to register a domain
            domain_register_fee: KAMBEI_PER_IPN / 10,
            // 0.05 IPN per year for domain renewal
            domain_renew_fee_per_year: KAMBEI_PER_IPN / 20,
        }
    }
}

impl RegistryPriceScheduleV1 {
    /// Compute handle registration fee (with optional premium multiplier)
    pub fn compute_handle_register_fee(&self, is_premium: bool) -> Kambei {
        if is_premium {
            // Apply premium multiplier using integer math
            let base = self.handle_register_fee as u128;
            let multiplied = base * self.premium_handle_multiplier_bps as u128 / 10000;
            multiplied as Kambei
        } else {
            self.handle_register_fee
        }
    }

    /// Compute handle renewal fee for a given number of years
    pub fn compute_handle_renew_fee(&self, years: u32) -> Kambei {
        self.handle_renew_fee_per_year.saturating_mul(years as u64)
    }

    /// Compute domain renewal fee for a given number of years
    pub fn compute_domain_renew_fee(&self, years: u32) -> Kambei {
        self.domain_renew_fee_per_year.saturating_mul(years as u64)
    }

    /// Validate minimum floors
    pub fn validate(&self) -> Result<(), FeeScheduleError> {
        if self.handle_register_fee < 1 {
            return Err(FeeScheduleError::RegistryFeeBelow1Kambei {
                field: "handle_register_fee".to_string(),
            });
        }
        if self.domain_register_fee < 1 {
            return Err(FeeScheduleError::RegistryFeeBelow1Kambei {
                field: "domain_register_fee".to_string(),
            });
        }
        Ok(())
    }
}

// =============================================================================
// ERRORS
// =============================================================================

/// Errors related to fee schedule validation
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeeScheduleError {
    #[error("tx_base_fee must be >= 1 kambei")]
    BaseFeeBelow1Kambei,

    #[error("tx_min_fee must be >= 1 kambei")]
    MinFeeBelow1Kambei,

    #[error("tx_max_fee must be >= tx_min_fee")]
    MaxFeeBelowMinFee,

    #[error("Registry fee {field} must be >= 1 kambei")]
    RegistryFeeBelow1Kambei { field: String },

    #[error("Rate limit exceeded for {field}: old={old}, new={new}, limit={max_allowed}")]
    RateLimitExceeded {
        field: String,
        old: u64,
        new: u64,
        max_allowed: u64,
    },
}

// =============================================================================
// FAILURE CHARGING POLICY
// =============================================================================

/// Categories of transaction failure for fee charging decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxFailureCategory {
    /// Parse failure - could not decode transaction bytes
    /// No fee charged (cannot identify sender)
    ParseFailure,

    /// Signature verification failed
    /// No fee charged (cannot verify authority)
    SignatureFailure,

    /// Invalid nonce (too old or too far in future)
    /// Charge minimum fee if sender can pay
    InvalidNonce,

    /// Insufficient balance for transfer + fee
    /// Charge minimum fee if sender can pay the fee
    InsufficientBalance,

    /// State conflict (e.g., double spend detected)
    /// Charge base fee if sender can pay
    StateConflict,

    /// Handle/domain registry operation failed
    /// Charge base fee if sender can pay
    RegistryFailure,

    /// Execution failure (contract reverted, etc.)
    /// Charge computed fee (gas consumed)
    ExecutionFailure,
}

impl TxFailureCategory {
    /// Determine if this failure category should charge a fee
    pub fn should_charge_fee(&self) -> bool {
        !matches!(
            self,
            TxFailureCategory::ParseFailure | TxFailureCategory::SignatureFailure
        )
    }

    /// Get the fee level to charge for this failure category
    pub fn fee_level(&self) -> FailureFeeLevel {
        match self {
            TxFailureCategory::ParseFailure => FailureFeeLevel::None,
            TxFailureCategory::SignatureFailure => FailureFeeLevel::None,
            TxFailureCategory::InvalidNonce => FailureFeeLevel::MinFee,
            TxFailureCategory::InsufficientBalance => FailureFeeLevel::MinFee,
            TxFailureCategory::StateConflict => FailureFeeLevel::BaseFee,
            TxFailureCategory::RegistryFailure => FailureFeeLevel::BaseFee,
            TxFailureCategory::ExecutionFailure => FailureFeeLevel::ComputedFee,
        }
    }
}

/// Level of fee to charge on failure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureFeeLevel {
    /// No fee charged
    None,
    /// Charge tx_min_fee
    MinFee,
    /// Charge tx_base_fee
    BaseFee,
    /// Charge the full computed fee
    ComputedFee,
}

// =============================================================================
// EPOCH DISTRIBUTION
// =============================================================================

/// State for tracking fee pool distribution within an epoch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochFeePoolState {
    /// Current epoch number
    pub epoch: Epoch,
    /// Total fees accumulated in this epoch (kambei)
    pub accumulated_fees: u128,
    /// Whether distribution has been executed for this epoch
    pub distributed: bool,
    /// Round at which distribution was executed (if distributed)
    pub distribution_round: Option<u64>,
}

impl EpochFeePoolState {
    /// Create a new fee pool state for the given epoch
    pub fn new(epoch: Epoch) -> Self {
        Self {
            epoch,
            accumulated_fees: 0,
            distributed: false,
            distribution_round: None,
        }
    }

    /// Add fees to the pool
    pub fn accumulate(&mut self, amount: Kambei) {
        self.accumulated_fees = self.accumulated_fees.saturating_add(amount as u128);
    }

    /// Mark distribution as complete
    pub fn mark_distributed(&mut self, round: u64) {
        self.distributed = true;
        self.distribution_round = Some(round);
    }
}

// =============================================================================
// PAYOUT PROFILE
// =============================================================================

/// Validator payout profile for splitting rewards among multiple addresses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutProfile {
    /// Up to 8 recipient addresses with their weights
    pub recipients: Vec<PayoutRecipient>,
}

/// A single payout recipient
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutRecipient {
    /// Recipient address (32 bytes)
    pub address: [u8; 32],
    /// Weight in basis points (must sum to 10000 across all recipients)
    pub weight_bps: u16,
}

impl PayoutProfile {
    /// Maximum number of recipients in a payout profile
    pub const MAX_RECIPIENTS: usize = 8;

    /// Create a single-recipient profile (100% to one address)
    pub fn single(address: [u8; 32]) -> Self {
        Self {
            recipients: vec![PayoutRecipient {
                address,
                weight_bps: 10000,
            }],
        }
    }

    /// Validate the payout profile
    pub fn validate(&self) -> Result<(), PayoutProfileError> {
        if self.recipients.is_empty() {
            return Err(PayoutProfileError::NoRecipients);
        }
        if self.recipients.len() > Self::MAX_RECIPIENTS {
            return Err(PayoutProfileError::TooManyRecipients {
                count: self.recipients.len(),
            });
        }

        let total_weight: u32 = self.recipients.iter().map(|r| r.weight_bps as u32).sum();
        if total_weight != 10000 {
            return Err(PayoutProfileError::WeightsSumNot10000 { sum: total_weight });
        }

        Ok(())
    }

    /// Split an amount according to the payout profile
    /// Returns (splits, remainder) where remainder is due to integer division
    pub fn split_amount(&self, total: Kambei) -> (Vec<(PayoutRecipient, Kambei)>, Kambei) {
        let mut splits = Vec::with_capacity(self.recipients.len());
        let mut distributed: Kambei = 0;

        for recipient in &self.recipients {
            let share = mul_div_u128(total as u128, recipient.weight_bps as u128, 10000)
                .unwrap_or(0) as Kambei;
            splits.push((recipient.clone(), share));
            distributed = distributed.saturating_add(share);
        }

        let remainder = total.saturating_sub(distributed);
        (splits, remainder)
    }
}

/// Errors for payout profile validation
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PayoutProfileError {
    #[error("Payout profile must have at least one recipient")]
    NoRecipients,

    #[error("Too many recipients: {count} > max {}", PayoutProfile::MAX_RECIPIENTS)]
    TooManyRecipients { count: usize },

    #[error("Recipient weights must sum to 10000 bps, got {sum}")]
    WeightsSumNot10000 { sum: u32 },
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kambei_conversions() {
        assert_eq!(ipn_to_kambei(1), KAMBEI_PER_IPN);
        assert_eq!(ipn_to_kambei(10), 10 * KAMBEI_PER_IPN);
        assert_eq!(kambei_to_ipn(KAMBEI_PER_IPN), 1);
        assert_eq!(kambei_to_ipn(KAMBEI_PER_IPN + 50_000_000), 1); // Truncates
    }

    #[test]
    fn test_epoch_from_time() {
        // Epoch 0: 0 to EPOCH_US - 1
        assert_eq!(epoch_from_ippan_time_us(0), 0);
        assert_eq!(epoch_from_ippan_time_us(EPOCH_US - 1), 0);
        // Epoch 1: EPOCH_US to 2*EPOCH_US - 1
        assert_eq!(epoch_from_ippan_time_us(EPOCH_US), 1);
        assert_eq!(epoch_from_ippan_time_us(2 * EPOCH_US - 1), 1);
        // Epoch 2
        assert_eq!(epoch_from_ippan_time_us(2 * EPOCH_US), 2);
    }

    #[test]
    fn test_epoch_boundaries() {
        assert_eq!(epoch_start_us(0), 0);
        assert_eq!(epoch_end_us(0), EPOCH_US);
        assert_eq!(epoch_start_us(1), EPOCH_US);
        assert_eq!(epoch_end_us(1), 2 * EPOCH_US);
    }

    #[test]
    fn test_fee_pool_account_id_determinism() {
        let id1 = fee_pool_account_id(0);
        let id2 = fee_pool_account_id(0);
        assert_eq!(id1, id2);

        let id3 = fee_pool_account_id(1);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_fee_schedule_compute() {
        let schedule = FeeScheduleV1::default();

        // Very small tx (0 bytes): should hit min_fee
        let fee_zero = schedule.compute_tx_fee(0);
        assert_eq!(fee_zero, schedule.tx_min_fee);

        // Small tx (10 bytes): base + bytes
        let fee_small = schedule.compute_tx_fee(10);
        let expected_small = schedule.tx_base_fee + schedule.tx_byte_fee * 10;
        assert_eq!(fee_small, expected_small);

        // Larger tx: base + bytes
        let fee_1000 = schedule.compute_tx_fee(1000);
        let expected = schedule.tx_base_fee + schedule.tx_byte_fee * 1000;
        assert_eq!(fee_1000, expected);
    }

    #[test]
    fn test_fee_schedule_validation() {
        let mut schedule = FeeScheduleV1::default();
        assert!(schedule.validate().is_ok());

        schedule.tx_base_fee = 0;
        assert!(matches!(
            schedule.validate(),
            Err(FeeScheduleError::BaseFeeBelow1Kambei)
        ));
    }

    #[test]
    fn test_rate_limits() {
        let old = FeeScheduleV1::default();
        let mut new = old;

        // Within limits (+10%)
        new.tx_base_fee = old.tx_base_fee * 11 / 10;
        assert!(new.validate_rate_limits(&old).is_ok());

        // Exceeds limits (+25%)
        new.tx_base_fee = old.tx_base_fee * 125 / 100;
        assert!(matches!(
            new.validate_rate_limits(&old),
            Err(FeeScheduleError::RateLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_registry_price_schedule() {
        let schedule = RegistryPriceScheduleV1::default();

        // Normal handle
        let normal_fee = schedule.compute_handle_register_fee(false);
        assert_eq!(normal_fee, schedule.handle_register_fee);

        // Premium handle (2x)
        let premium_fee = schedule.compute_handle_register_fee(true);
        assert_eq!(premium_fee, schedule.handle_register_fee * 2);

        // Renewal for 3 years
        let renew_3y = schedule.compute_handle_renew_fee(3);
        assert_eq!(renew_3y, schedule.handle_renew_fee_per_year * 3);
    }

    #[test]
    fn test_payout_profile_single() {
        let addr = [1u8; 32];
        let profile = PayoutProfile::single(addr);
        assert!(profile.validate().is_ok());

        let (splits, remainder) = profile.split_amount(1000);
        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].1, 1000);
        assert_eq!(remainder, 0);
    }

    #[test]
    fn test_payout_profile_multi() {
        let profile = PayoutProfile {
            recipients: vec![
                PayoutRecipient {
                    address: [1u8; 32],
                    weight_bps: 5000,
                }, // 50%
                PayoutRecipient {
                    address: [2u8; 32],
                    weight_bps: 3000,
                }, // 30%
                PayoutRecipient {
                    address: [3u8; 32],
                    weight_bps: 2000,
                }, // 20%
            ],
        };
        assert!(profile.validate().is_ok());

        let (splits, remainder) = profile.split_amount(1000);
        assert_eq!(splits.len(), 3);
        assert_eq!(splits[0].1, 500); // 50%
        assert_eq!(splits[1].1, 300); // 30%
        assert_eq!(splits[2].1, 200); // 20%
        assert_eq!(remainder, 0);
    }

    #[test]
    fn test_payout_profile_remainder() {
        let profile = PayoutProfile {
            recipients: vec![
                PayoutRecipient {
                    address: [1u8; 32],
                    weight_bps: 3333,
                },
                PayoutRecipient {
                    address: [2u8; 32],
                    weight_bps: 3333,
                },
                PayoutRecipient {
                    address: [3u8; 32],
                    weight_bps: 3334,
                },
            ],
        };
        assert!(profile.validate().is_ok());

        let (splits, remainder) = profile.split_amount(100);
        let total_splits: Kambei = splits.iter().map(|s| s.1).sum();
        assert_eq!(total_splits + remainder, 100);
    }

    #[test]
    fn test_payout_profile_validation() {
        // Empty recipients
        let empty = PayoutProfile { recipients: vec![] };
        assert!(matches!(
            empty.validate(),
            Err(PayoutProfileError::NoRecipients)
        ));

        // Wrong weights
        let wrong_weights = PayoutProfile {
            recipients: vec![PayoutRecipient {
                address: [1u8; 32],
                weight_bps: 5000,
            }],
        };
        assert!(matches!(
            wrong_weights.validate(),
            Err(PayoutProfileError::WeightsSumNot10000 { sum: 5000 })
        ));
    }

    #[test]
    fn test_mul_div_u128() {
        assert_eq!(mul_div_u128(100, 50, 100), Some(50));
        assert_eq!(mul_div_u128(1000, 3333, 10000), Some(333));
        assert_eq!(mul_div_u128(100, 1, 0), None); // Division by zero
    }

    #[test]
    fn test_failure_category_fee_levels() {
        assert!(!TxFailureCategory::ParseFailure.should_charge_fee());
        assert!(!TxFailureCategory::SignatureFailure.should_charge_fee());
        assert!(TxFailureCategory::InvalidNonce.should_charge_fee());
        assert!(TxFailureCategory::InsufficientBalance.should_charge_fee());

        assert_eq!(
            TxFailureCategory::InvalidNonce.fee_level(),
            FailureFeeLevel::MinFee
        );
        assert_eq!(
            TxFailureCategory::StateConflict.fee_level(),
            FailureFeeLevel::BaseFee
        );
    }

    #[test]
    fn test_epoch_fee_pool_state() {
        let mut state = EpochFeePoolState::new(5);
        assert_eq!(state.epoch, 5);
        assert_eq!(state.accumulated_fees, 0);
        assert!(!state.distributed);

        state.accumulate(1000);
        state.accumulate(500);
        assert_eq!(state.accumulated_fees, 1500);

        state.mark_distributed(12345);
        assert!(state.distributed);
        assert_eq!(state.distribution_round, Some(12345));
    }
}
