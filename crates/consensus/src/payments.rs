use anyhow::Error as AnyError;
use ippan_l1_fees::{FeePolicy, FeeSplit};
use ippan_storage::{Account, Storage};
use ippan_types::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Canonical treasury account used for L1 fee recycling.
pub const TREASURY_ACCOUNT: [u8; 32] = [0u8; 32];

/// Applies deterministic payment fees and balance updates against canonical storage.
#[derive(Debug, Clone)]
pub struct PaymentApplier {
    policy: FeePolicy,
    treasury_account: [u8; 32],
}

impl PaymentApplier {
    pub fn new(policy: FeePolicy, treasury_account: [u8; 32]) -> Self {
        Self {
            policy,
            treasury_account,
        }
    }

    pub fn policy(&self) -> &FeePolicy {
        &self.policy
    }

    pub fn apply(
        &self,
        storage: &Arc<dyn Storage + Send + Sync>,
        tx: &Transaction,
        proposer: &[u8; 32],
    ) -> Result<FeeSplit, PaymentApplyError> {
        let mut sender = storage
            .get_account(&tx.from)
            .map_err(PaymentApplyError::Storage)?
            .ok_or(PaymentApplyError::MissingAccount(tx.from))?;

        let expected_nonce = sender.nonce.saturating_add(1);
        if tx.nonce != expected_nonce {
            return Err(PaymentApplyError::NonceMismatch {
                expected: expected_nonce,
                got: tx.nonce,
            });
        }

        let amount_atomic = tx.amount.atomic();
        let fee_atomic = self.policy.required_fee(tx) as u128;
        let total_cost = amount_atomic
            .checked_add(fee_atomic)
            .ok_or(PaymentApplyError::BalanceOverflow)?;

        let sender_balance = sender.balance as u128;
        if sender_balance < total_cost {
            return Err(PaymentApplyError::InsufficientBalance {
                available: sender_balance,
                required: total_cost,
            });
        }

        let updated_sender_balance = sender_balance - total_cost;
        sender.balance = updated_sender_balance
            .try_into()
            .map_err(|_| PaymentApplyError::BalanceOverflow)?;
        sender.nonce = tx.nonce;
        storage
            .update_account(sender)
            .map_err(PaymentApplyError::Storage)?;

        credit_account(storage, &tx.to, amount_atomic)?;

        let split = self.policy.split_fee(fee_atomic);
        credit_account(storage, proposer, split.validator_fee)?;
        credit_account(storage, &self.treasury_account, split.treasury_fee)?;

        Ok(split)
    }
}

fn credit_account(
    storage: &Arc<dyn Storage + Send + Sync>,
    address: &[u8; 32],
    amount_atomic: u128,
) -> Result<(), PaymentApplyError> {
    if amount_atomic == 0 {
        return Ok(());
    }

    let mut account = storage
        .get_account(address)
        .map_err(PaymentApplyError::Storage)?
        .unwrap_or(Account {
            address: *address,
            balance: 0,
            nonce: 0,
        });

    let new_balance = (account.balance as u128)
        .checked_add(amount_atomic)
        .ok_or(PaymentApplyError::BalanceOverflow)?;
    account.balance = new_balance
        .try_into()
        .map_err(|_| PaymentApplyError::BalanceOverflow)?;
    storage
        .update_account(account)
        .map_err(PaymentApplyError::Storage)?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum PaymentApplyError {
    #[error("account {0:?} not found")]
    MissingAccount([u8; 32]),
    #[error("nonce mismatch (expected {expected}, got {got})")]
    NonceMismatch { expected: u64, got: u64 },
    #[error("insufficient balance: have {available}, need {required}")]
    InsufficientBalance { available: u128, required: u128 },
    #[error("balance overflow detected during update")]
    BalanceOverflow,
    #[error("storage error: {0}")]
    Storage(AnyError),
}

impl PaymentApplyError {
    pub fn kind(&self) -> PaymentApplyErrorKind {
        match self {
            PaymentApplyError::MissingAccount(_) => PaymentApplyErrorKind::MissingAccount,
            PaymentApplyError::NonceMismatch { .. } => PaymentApplyErrorKind::NonceMismatch,
            PaymentApplyError::InsufficientBalance { .. } => {
                PaymentApplyErrorKind::InsufficientBalance
            }
            PaymentApplyError::BalanceOverflow => PaymentApplyErrorKind::BalanceOverflow,
            PaymentApplyError::Storage(_) => PaymentApplyErrorKind::Storage,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentApplyErrorKind {
    MissingAccount,
    NonceMismatch,
    InsufficientBalance,
    BalanceOverflow,
    Storage,
}

#[derive(Debug, Default)]
pub struct PaymentRoundStats {
    pub round: u64,
    pub applied: usize,
    pub rejected: usize,
    pub total_amount: u128,
    pub total_fees: u128,
    pub treasury_total: u128,
    pub validator_fees: HashMap<[u8; 32], u128>,
    pub failure_counts: HashMap<PaymentApplyErrorKind, usize>,
}

impl PaymentRoundStats {
    pub fn new(round: u64) -> Self {
        Self {
            round,
            ..Default::default()
        }
    }

    pub fn record_success(&mut self, tx: &Transaction, proposer: [u8; 32], split: FeeSplit) {
        self.applied += 1;
        self.total_amount = self.total_amount.saturating_add(tx.amount.atomic());
        self.total_fees = self.total_fees.saturating_add(split.total_fee);
        self.treasury_total = self.treasury_total.saturating_add(split.treasury_fee);
        *self.validator_fees.entry(proposer).or_insert(0) += split.validator_fee;
    }

    pub fn record_failure(&mut self, error: &PaymentApplyError) {
        self.rejected += 1;
        *self.failure_counts.entry(error.kind()).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_storage::{Account, MemoryStorage};
    use ippan_types::{Amount, Transaction};

    fn sample_transaction(amount: u128, nonce: u64) -> Transaction {
        let sender = [1u8; 32];
        let receiver = [2u8; 32];
        Transaction::new(sender, receiver, Amount::from_atomic(amount), nonce)
    }

    #[test]
    fn apply_payment_updates_balances() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let sender_account = Account {
            address: [1u8; 32],
            balance: 10_000,
            nonce: 0,
        };
        storage
            .update_account(sender_account)
            .expect("update sender");

        let tx = sample_transaction(1_000, 1);
        let proposer = [9u8; 32];
        let applier = PaymentApplier::new(FeePolicy::default(), TREASURY_ACCOUNT);

        let split = applier
            .apply(&storage, &tx, &proposer)
            .expect("apply payment");
        assert!(split.total_fee > 0);

        let sender_after = storage
            .get_account(&tx.from)
            .expect("sender fetch")
            .expect("sender");
        assert_eq!(sender_after.nonce, 1);
        assert!(sender_after.balance < 10_000);

        let receiver = storage
            .get_account(&tx.to)
            .expect("receiver fetch")
            .expect("receiver");
        assert_eq!(receiver.balance, 1_000);

        let validator = storage
            .get_account(&proposer)
            .expect("validator fetch")
            .expect("validator");
        assert_eq!(validator.balance as u128, split.validator_fee);

        let treasury = storage
            .get_account(&TREASURY_ACCOUNT)
            .expect("treasury fetch")
            .expect("treasury");
        assert_eq!(treasury.balance as u128, split.treasury_fee);
    }

    #[test]
    fn apply_payment_detects_insufficient_balance() {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(MemoryStorage::new());
        let sender_account = Account {
            address: [1u8; 32],
            balance: 10,
            nonce: 0,
        };
        storage
            .update_account(sender_account)
            .expect("update sender");

        let tx = sample_transaction(5_000, 1);
        let proposer = [9u8; 32];
        let applier = PaymentApplier::new(FeePolicy::default(), TREASURY_ACCOUNT);

        let err = applier
            .apply(&storage, &tx, &proposer)
            .expect_err("insufficient");
        assert!(matches!(err, PaymentApplyError::InsufficientBalance { .. }));
    }
}
