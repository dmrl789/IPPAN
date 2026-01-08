//! Weekly Fee Pool — Deterministic Epoch-Based Fee Redistribution
//!
//! This module implements the protocol-owned weekly fee pool that:
//! - Accumulates all transaction and registry fees during an epoch
//! - Provides deterministic pool account IDs (no private key)
//! - Distributes fees to eligible validators at epoch close
//! - Ensures fees are NEVER burned — all go to participants
//!
//! ## Key Invariants
//! - Fees can only leave the pool via `DistributeFees` transition
//! - Distribution can only run once per epoch
//! - Remainder goes to next epoch pool (not burned)
//! - All calculations use integer math (no floats)

use crate::account_ledger::AccountLedger;
use anyhow::{anyhow, Result};
use ippan_types::{
    epoch_from_ippan_time_us, fee_pool_account_id, mul_div_u128, Epoch, EpochFeePoolState, Kambei,
    PayoutProfile, ValidatorId,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

#[cfg(test)]
use ippan_types::EPOCH_US;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Minimum presence score (basis points, 10000 = 100%) required for epoch eligibility.
/// Per policy: validators must have UNINTERRUPTED presence to be eligible.
/// This means 100% presence (10000 bps).
pub const MIN_PRESENCE_BPS: u16 = 10000; // 100% - uninterrupted presence required

/// Work score cap to prevent whale dominance
pub const WORK_SCORE_CAP: u64 = 10_000;

// =============================================================================
// VALIDATOR ELIGIBILITY
// =============================================================================

/// Metrics used to determine validator eligibility and weight for fee distribution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorEpochMetrics {
    /// Validator identifier
    pub validator_id: ValidatorId,
    /// Presence score in basis points (0-10000 = 0%-100%)
    pub presence_bps: u16,
    /// Work score: blocks verified, receipts processed, etc.
    pub work_score: u64,
    /// Whether validator had uninterrupted presence during the epoch.
    /// This is the PRIMARY eligibility criterion per policy.
    pub uninterrupted_presence: bool,
    /// Whether validator was slashed during this epoch
    pub slashed: bool,
}

impl ValidatorEpochMetrics {
    /// Create new metrics with strict uninterrupted presence check
    pub fn new(validator_id: ValidatorId, presence_bps: u16, work_score: u64) -> Self {
        // Uninterrupted means 100% presence (10000 bps)
        let uninterrupted = presence_bps >= MIN_PRESENCE_BPS;
        Self {
            validator_id,
            presence_bps,
            work_score,
            uninterrupted_presence: uninterrupted,
            slashed: false,
        }
    }

    /// Create metrics with explicit uninterrupted flag (for when presence tracking is binary)
    pub fn with_uninterrupted(
        validator_id: ValidatorId,
        uninterrupted: bool,
        work_score: u64,
    ) -> Self {
        Self {
            validator_id,
            presence_bps: if uninterrupted { 10000 } else { 0 },
            work_score,
            uninterrupted_presence: uninterrupted,
            slashed: false,
        }
    }

    /// Mark this validator as slashed
    pub fn mark_slashed(&mut self) {
        self.slashed = true;
    }

    /// Check if validator is eligible for fee distribution.
    /// Per policy: requires uninterrupted presence AND not slashed AND work_score > 0
    pub fn is_eligible(&self) -> bool {
        self.uninterrupted_presence && !self.slashed && self.work_score > 0
    }

    /// Calculate this validator's weight for distribution
    /// Weight = min(work_score, WORK_SCORE_CAP) if eligible, else 0
    /// Note: presence weight is binary (1 if uninterrupted, 0 otherwise)
    pub fn weight(&self) -> u64 {
        if !self.is_eligible() {
            return 0;
        }
        self.work_score.min(WORK_SCORE_CAP)
    }
}

/// Result of validator eligibility assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityResult {
    /// Eligible validators with their weights
    pub eligible: Vec<(ValidatorId, u64)>,
    /// Validators that didn't meet presence threshold
    pub ineligible: Vec<ValidatorId>,
    /// Total weight of all eligible validators
    pub total_weight: u128,
}

/// Assess validator eligibility for a given set of metrics
pub fn assess_eligibility(metrics: &[ValidatorEpochMetrics]) -> EligibilityResult {
    let mut eligible = Vec::new();
    let mut ineligible = Vec::new();
    let mut total_weight: u128 = 0;

    for m in metrics {
        let w = m.weight();
        if w > 0 {
            eligible.push((m.validator_id, w));
            total_weight = total_weight.saturating_add(w as u128);
        } else {
            ineligible.push(m.validator_id);
        }
    }

    EligibilityResult {
        eligible,
        ineligible,
        total_weight,
    }
}

// =============================================================================
// FEE POOL MANAGER
// =============================================================================

/// Manages the weekly fee pool accumulation and distribution
#[derive(Debug)]
pub struct WeeklyFeePoolManager {
    /// Current epoch state
    current_state: RwLock<EpochFeePoolState>,
    /// Historical epoch states (for audit/replay)
    history: RwLock<HashMap<Epoch, EpochFeePoolState>>,
    /// Validator payout profiles
    payout_profiles: RwLock<HashMap<ValidatorId, PayoutProfile>>,
}

impl WeeklyFeePoolManager {
    /// Create a new pool manager starting at the given epoch
    pub fn new(current_epoch: Epoch) -> Self {
        Self {
            current_state: RwLock::new(EpochFeePoolState::new(current_epoch)),
            history: RwLock::new(HashMap::new()),
            payout_profiles: RwLock::new(HashMap::new()),
        }
    }

    /// Create from IPPAN time in microseconds
    pub fn from_ippan_time_us(time_us: u64) -> Self {
        let epoch = epoch_from_ippan_time_us(time_us);
        Self::new(epoch)
    }

    /// Get the current epoch
    pub fn current_epoch(&self) -> Epoch {
        self.current_state.read().epoch
    }

    /// Get the pool account ID for the current epoch
    pub fn current_pool_account(&self) -> [u8; 32] {
        fee_pool_account_id(self.current_epoch())
    }

    /// Get the pool account ID for a specific epoch
    pub fn pool_account_for_epoch(&self, epoch: Epoch) -> [u8; 32] {
        fee_pool_account_id(epoch)
    }

    /// Accumulate fees into the current epoch's pool
    pub fn accumulate_fee(&self, amount: Kambei) {
        let mut state = self.current_state.write();
        state.accumulate(amount);
        debug!(
            epoch = state.epoch,
            amount = amount,
            total = state.accumulated_fees,
            "Fee accumulated into weekly pool"
        );
    }

    /// Get total accumulated fees for current epoch
    pub fn accumulated_fees(&self) -> u128 {
        self.current_state.read().accumulated_fees
    }

    /// Check if the current epoch has been distributed
    pub fn is_distributed(&self) -> bool {
        self.current_state.read().distributed
    }

    /// Register a validator's payout profile
    pub fn register_payout_profile(
        &self,
        validator_id: ValidatorId,
        profile: PayoutProfile,
    ) -> Result<()> {
        profile.validate()?;
        self.payout_profiles.write().insert(validator_id, profile);
        info!(
            validator = ?validator_id,
            "Registered payout profile"
        );
        Ok(())
    }

    /// Get a validator's payout profile (or create default single-recipient)
    pub fn get_payout_profile(&self, validator_id: &ValidatorId) -> PayoutProfile {
        self.payout_profiles
            .read()
            .get(validator_id)
            .cloned()
            .unwrap_or_else(|| PayoutProfile::single(*validator_id))
    }

    /// Advance to the next epoch (should be called at epoch boundary)
    pub fn advance_epoch(&self, new_epoch: Epoch) -> Result<()> {
        let mut state = self.current_state.write();

        if new_epoch <= state.epoch {
            return Err(anyhow!(
                "Cannot advance to epoch {} from {}",
                new_epoch,
                state.epoch
            ));
        }

        // Archive current state
        let old_state = state.clone();
        self.history.write().insert(old_state.epoch, old_state);

        // Initialize new epoch
        *state = EpochFeePoolState::new(new_epoch);

        info!(epoch = new_epoch, "Advanced to new epoch");
        Ok(())
    }

    /// Execute fee distribution for the current epoch
    ///
    /// This is the ONLY way funds can leave the pool account.
    /// Returns the distribution result including any remainder.
    pub fn distribute_fees(
        &self,
        metrics: &[ValidatorEpochMetrics],
        round: u64,
        ledger: &mut dyn AccountLedger,
    ) -> Result<DistributionResult> {
        let mut state = self.current_state.write();

        // Check invariant: can only distribute once
        if state.distributed {
            return Err(anyhow!(
                "Epoch {} already distributed at round {:?}",
                state.epoch,
                state.distribution_round
            ));
        }

        let pool_balance = state.accumulated_fees;
        if pool_balance == 0 {
            state.mark_distributed(round);
            return Ok(DistributionResult {
                epoch: state.epoch,
                pool_balance: 0,
                total_distributed: 0,
                remainder: 0,
                payouts: vec![],
            });
        }

        // Assess eligibility
        let eligibility = assess_eligibility(metrics);
        if eligibility.total_weight == 0 {
            // No eligible validators - remainder carries to next epoch
            info!(
                epoch = state.epoch,
                pool_balance = pool_balance,
                "No eligible validators, carrying balance to next epoch"
            );
            state.mark_distributed(round);
            return Ok(DistributionResult {
                epoch: state.epoch,
                pool_balance,
                total_distributed: 0,
                remainder: pool_balance as Kambei,
                payouts: vec![],
            });
        }

        // Compute payouts
        let mut payouts = Vec::new();
        let mut total_distributed: u128 = 0;

        for (validator_id, weight) in &eligibility.eligible {
            // payout = pool_balance * weight / total_weight
            let payout = mul_div_u128(pool_balance, *weight as u128, eligibility.total_weight)
                .ok_or_else(|| anyhow!("Division error in payout calculation"))?;

            if payout > 0 {
                // Get payout profile and apply splits
                let profile = self.get_payout_profile(validator_id);
                let (splits, split_remainder) = profile.split_amount(payout as Kambei);

                for (recipient, amount) in splits {
                    if amount > 0 {
                        // Credit to ledger
                        ledger.credit_validator(&recipient.address, amount as u128)?;
                        payouts.push(ValidatorPayout {
                            validator_id: *validator_id,
                            recipient: recipient.address,
                            amount,
                            weight: *weight,
                        });
                    }
                }

                total_distributed =
                    total_distributed.saturating_add(payout - split_remainder as u128);
            }
        }

        // Remainder goes to next epoch pool
        let remainder = pool_balance.saturating_sub(total_distributed);

        state.mark_distributed(round);

        info!(
            epoch = state.epoch,
            pool_balance = pool_balance,
            total_distributed = total_distributed,
            remainder = remainder,
            num_payouts = payouts.len(),
            "Fee distribution complete"
        );

        Ok(DistributionResult {
            epoch: state.epoch,
            pool_balance,
            total_distributed,
            remainder: remainder as Kambei,
            payouts,
        })
    }

    /// Get historical state for a specific epoch
    pub fn get_epoch_state(&self, epoch: Epoch) -> Option<EpochFeePoolState> {
        if self.current_state.read().epoch == epoch {
            Some(self.current_state.read().clone())
        } else {
            self.history.read().get(&epoch).cloned()
        }
    }

    /// Get statistics about the fee pool
    pub fn get_statistics(&self) -> PoolStatistics {
        let current = self.current_state.read().clone();
        let history = self.history.read();

        let total_distributed: u128 = history
            .values()
            .filter(|s| s.distributed)
            .map(|s| s.accumulated_fees)
            .sum();

        PoolStatistics {
            current_epoch: current.epoch,
            current_accumulated: current.accumulated_fees,
            is_distributed: current.distributed,
            historical_epochs: history.len(),
            total_distributed,
        }
    }
}

impl Default for WeeklyFeePoolManager {
    fn default() -> Self {
        Self::new(0)
    }
}

// =============================================================================
// DISTRIBUTION RESULT
// =============================================================================

/// Result of a fee distribution operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    /// Epoch that was distributed
    pub epoch: Epoch,
    /// Total pool balance at distribution time
    pub pool_balance: u128,
    /// Total amount distributed to validators
    pub total_distributed: u128,
    /// Remainder carried to next epoch (due to integer division)
    pub remainder: Kambei,
    /// Individual validator payouts
    pub payouts: Vec<ValidatorPayout>,
}

/// Individual validator payout details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorPayout {
    /// Original validator who earned the reward
    pub validator_id: ValidatorId,
    /// Recipient address (from payout profile)
    pub recipient: [u8; 32],
    /// Amount paid (kambei)
    pub amount: Kambei,
    /// Weight used in calculation
    pub weight: u64,
}

/// Pool statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub current_epoch: Epoch,
    pub current_accumulated: u128,
    pub is_distributed: bool,
    pub historical_epochs: usize,
    pub total_distributed: u128,
}

// =============================================================================
// DISTRIBUTE FEES TRANSITION
// =============================================================================

/// Protocol transition for fee distribution
///
/// This is executed by the consensus layer at epoch boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributeFeesTransition {
    /// Target epoch for distribution
    pub epoch: Epoch,
    /// Round at which transition is executed
    pub execution_round: u64,
}

impl DistributeFeesTransition {
    /// Create a new distribution transition
    pub fn new(epoch: Epoch, execution_round: u64) -> Self {
        Self {
            epoch,
            execution_round,
        }
    }

    /// Validate that this transition can be executed
    pub fn validate(&self, current_epoch: Epoch, is_distributed: bool) -> Result<()> {
        if self.epoch != current_epoch {
            return Err(anyhow!(
                "Transition epoch {} does not match current epoch {}",
                self.epoch,
                current_epoch
            ));
        }
        if is_distributed {
            return Err(anyhow!("Epoch {} already distributed", self.epoch));
        }
        Ok(())
    }
}

// =============================================================================
// FEE ROUTING HELPERS
// =============================================================================

/// Route a transaction fee to the weekly pool
///
/// This is called during transaction execution to credit fees
/// to the deterministic pool account for the current epoch.
pub fn route_fee_to_pool(
    pool_manager: &WeeklyFeePoolManager,
    fee: Kambei,
    ledger: &mut dyn AccountLedger,
) -> Result<()> {
    if fee == 0 {
        return Ok(());
    }

    // Get current pool account
    let pool_account = pool_manager.current_pool_account();

    // Credit fee to pool account in ledger
    ledger.credit_validator(&pool_account, fee as u128)?;

    // Track in pool manager
    pool_manager.accumulate_fee(fee);

    Ok(())
}

/// Determine if we should trigger epoch distribution
pub fn should_trigger_distribution(current_time_us: u64, last_distribution_time_us: u64) -> bool {
    let current_epoch = epoch_from_ippan_time_us(current_time_us);
    let last_epoch = epoch_from_ippan_time_us(last_distribution_time_us);
    current_epoch > last_epoch
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account_ledger::MockAccountLedger;

    fn test_validator_id(seed: u8) -> ValidatorId {
        let mut id = [0u8; 32];
        id[0] = seed;
        id
    }

    #[test]
    fn test_validator_eligibility_uninterrupted() {
        let metrics = vec![
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 500), // Eligible
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(2), false, 1000), // Not uninterrupted
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(3), true, 200), // Eligible
        ];

        let result = assess_eligibility(&metrics);
        assert_eq!(result.eligible.len(), 2);
        assert_eq!(result.ineligible.len(), 1);
    }

    #[test]
    fn test_validator_99_percent_ineligible() {
        // Per policy: 99% presence is NOT uninterrupted - must be 100%
        let metrics = ValidatorEpochMetrics::new(test_validator_id(1), 9900, 500); // 99%
        assert!(!metrics.is_eligible());
        assert_eq!(metrics.weight(), 0);

        // 100% presence IS eligible
        let metrics_100 = ValidatorEpochMetrics::new(test_validator_id(2), 10000, 500);
        assert!(metrics_100.is_eligible());
        assert!(metrics_100.weight() > 0);
    }

    #[test]
    fn test_slashed_validator_ineligible() {
        let mut metrics =
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 500);
        assert!(metrics.is_eligible());

        metrics.mark_slashed();
        assert!(!metrics.is_eligible());
        assert_eq!(metrics.weight(), 0);
    }

    #[test]
    fn test_zero_work_ineligible() {
        // Uninterrupted but no work = ineligible
        let metrics = ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 0);
        assert!(!metrics.is_eligible());
        assert_eq!(metrics.weight(), 0);
    }

    #[test]
    fn test_work_score_cap() {
        // Work score above cap should be capped
        let metrics =
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 100_000);
        assert_eq!(metrics.weight(), WORK_SCORE_CAP);
    }

    #[test]
    fn test_fee_accumulation() {
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(1000);
        manager.accumulate_fee(500);
        assert_eq!(manager.accumulated_fees(), 1500);
    }

    #[test]
    fn test_pool_account_determinism() {
        let manager = WeeklyFeePoolManager::new(5);
        let account1 = manager.pool_account_for_epoch(5);
        let account2 = manager.pool_account_for_epoch(5);
        assert_eq!(account1, account2);

        let account3 = manager.pool_account_for_epoch(6);
        assert_ne!(account1, account3);
    }

    #[test]
    fn test_epoch_advance() {
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(1000);

        manager.advance_epoch(1).unwrap();
        assert_eq!(manager.current_epoch(), 1);
        assert_eq!(manager.accumulated_fees(), 0); // New epoch starts fresh

        // Cannot go backwards
        assert!(manager.advance_epoch(0).is_err());
    }

    #[test]
    fn test_fee_distribution() {
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(10000);

        // Both validators with uninterrupted presence and equal work
        let metrics = vec![
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 500),
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(2), true, 500),
        ];

        let mut ledger = MockAccountLedger::new();
        let result = manager.distribute_fees(&metrics, 100, &mut ledger).unwrap();

        assert_eq!(result.epoch, 0);
        assert_eq!(result.pool_balance, 10000);
        // Each validator should get 50% (equal weight)
        assert_eq!(result.payouts.len(), 2);
        assert_eq!(result.payouts[0].amount, 5000);
        assert_eq!(result.payouts[1].amount, 5000);
        assert_eq!(result.total_distributed, 10000);
        assert_eq!(result.remainder, 0);
    }

    #[test]
    fn test_distribution_once_per_epoch() {
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(1000);

        let metrics = vec![ValidatorEpochMetrics::with_uninterrupted(
            test_validator_id(1),
            true,
            100,
        )];

        let mut ledger = MockAccountLedger::new();
        manager.distribute_fees(&metrics, 100, &mut ledger).unwrap();

        // Second distribution should fail
        assert!(manager.distribute_fees(&metrics, 101, &mut ledger).is_err());
    }

    #[test]
    fn test_payout_profile_splits() {
        let manager = WeeklyFeePoolManager::new(0);
        let validator = test_validator_id(1);

        // Register split profile: 60%/40%
        let profile = PayoutProfile {
            recipients: vec![
                ippan_types::PayoutRecipient {
                    address: [1u8; 32],
                    weight_bps: 6000,
                },
                ippan_types::PayoutRecipient {
                    address: [2u8; 32],
                    weight_bps: 4000,
                },
            ],
        };
        manager.register_payout_profile(validator, profile).unwrap();

        manager.accumulate_fee(1000);

        let metrics = vec![ValidatorEpochMetrics::with_uninterrupted(
            validator, true, 100,
        )];

        let mut ledger = MockAccountLedger::new();
        let result = manager.distribute_fees(&metrics, 100, &mut ledger).unwrap();

        // Should have 2 payouts from split
        assert_eq!(result.payouts.len(), 2);
        assert_eq!(result.payouts[0].amount, 600); // 60%
        assert_eq!(result.payouts[1].amount, 400); // 40%
    }

    #[test]
    fn test_no_eligible_validators() {
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(1000);

        // None with uninterrupted presence
        let metrics = vec![
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), false, 500),
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(2), false, 1000),
        ];

        let mut ledger = MockAccountLedger::new();
        let result = manager.distribute_fees(&metrics, 100, &mut ledger).unwrap();

        // All fees become remainder
        assert_eq!(result.total_distributed, 0);
        assert_eq!(result.remainder, 1000);
        assert!(result.payouts.is_empty());
    }

    #[test]
    fn test_distribution_remainder() {
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(100);

        // Weights that don't divide evenly (all uninterrupted)
        let metrics = vec![
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 33),
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(2), true, 33),
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(3), true, 34),
        ];

        let mut ledger = MockAccountLedger::new();
        let result = manager.distribute_fees(&metrics, 100, &mut ledger).unwrap();

        // Total distributed + remainder should equal pool balance
        let total = result.total_distributed + result.remainder as u128;
        assert_eq!(total, result.pool_balance);
    }

    #[test]
    fn test_distribution_sum_correctness() {
        // Invariant: sum(payouts) + remainder == pool_balance
        let manager = WeeklyFeePoolManager::new(0);
        manager.accumulate_fee(12345);

        let metrics = vec![
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(1), true, 100),
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(2), true, 200),
            ValidatorEpochMetrics::with_uninterrupted(test_validator_id(3), true, 300),
        ];

        let mut ledger = MockAccountLedger::new();
        let result = manager.distribute_fees(&metrics, 100, &mut ledger).unwrap();

        let sum_payouts: u64 = result.payouts.iter().map(|p| p.amount).sum();
        assert_eq!(
            sum_payouts as u128 + result.remainder as u128,
            result.pool_balance,
            "sum(payouts) + remainder must equal pool_balance"
        );
    }

    #[test]
    fn test_route_fee_to_pool() {
        let manager = WeeklyFeePoolManager::new(0);
        let mut ledger = MockAccountLedger::new();

        route_fee_to_pool(&manager, 500, &mut ledger).unwrap();

        assert_eq!(manager.accumulated_fees(), 500);
        // Ledger should have credited pool account
        let credits = ledger.get_credit_calls();
        assert_eq!(credits.len(), 1);
        assert_eq!(credits[0].0, manager.current_pool_account());
        assert_eq!(credits[0].1, 500);
    }

    #[test]
    fn test_should_trigger_distribution() {
        // Within same epoch
        assert!(!should_trigger_distribution(1000, 500));

        // Different epochs (assuming EPOCH_US separation)
        let epoch1_time = 1000;
        let epoch2_time = EPOCH_US + 1000;
        assert!(should_trigger_distribution(epoch2_time, epoch1_time));
    }
}
