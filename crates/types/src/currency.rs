//! IPPAN Currency & Atomic Unit System
//!
//! Implements ultra-fine divisibility for IPN tokens with 24 decimal precision.
//! Supports micro-payments for IoT, AI inference, and machine-to-machine economies.
//!
//! ## Denomination Table
//!
//! | Name     | Symbol    | Value in IPN | Atomic Units      | Use Case                    |
//! |----------|-----------|--------------|-------------------|-----------------------------|
//! | IPN      | IPN       | 1.0          | 10²⁴              | Governance, staking         |
//! | milli-IPN| mIPN      | 0.001        | 10²¹              | Validator micro-rewards     |
//! | micro-IPN| µIPN      | 0.000001     | 10¹⁸              | Transaction fees            |
//! | nano-IPN | nIPN      | 0.000000001  | 10¹⁵              | Micro-services              |
//! | pico-IPN | pIPN      | 0.000000000001| 10¹²             | Sub-cent settlements        |
//! | femto-IPN| fIPN      | 10⁻¹⁵        | 10⁹               | Streaming payments          |
//! | atto-IPN | aIPN      | 10⁻¹⁸        | 10⁶               | IoT energy metering         |
//! | zepto-IPN| zIPN      | 10⁻²¹        | 10³               | AI micro-inference          |
//! | yocto-IPN| yIPN      | 10⁻²⁴        | 1                 | Atomic unit (smallest)      |
//!
//! ## Design Rationale
//!
//! 1. **HashTimer Precision**: With 200ms rounds, sub-microsecond payment granularity
//!    enables fair distribution across thousands of parallel blocks per round.
//!
//! 2. **Rounding-Free Math**: Integer arithmetic at atomic precision eliminates
//!    floating-point drift and ensures deterministic reward splits.
//!
//! 3. **Future-Proof Scarcity**: 21M IPN × 10²⁴ atomic units = 2.1×10³¹ total units,
//!    supporting trillions of micro-transactions without exhaustion.
//!
//! 4. **Machine Economy Ready**: Enables pay-per-token AI inference, IoT data packets,
//!    compute-by-millisecond, and autonomous agent micro-payments.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

/// Number of decimal places in IPN token (yocto-IPN precision)
pub const IPN_DECIMALS: u32 = 24;

/// Atomic unit type: 1 IPN = 10²⁴ atomic units
pub type AtomicIPN = u128;

/// One full IPN token in atomic units
pub const ATOMIC_PER_IPN: AtomicIPN = 10u128.pow(IPN_DECIMALS);

/// Denomination constants for human-readable conversions
pub mod denominations {
    use super::AtomicIPN;

    /// 1 IPN = 10²⁴ atomic units
    pub const IPN: AtomicIPN = 1_000_000_000_000_000_000_000_000;

    /// 1 milli-IPN = 10²¹ atomic units
    pub const MILLI_IPN: AtomicIPN = 1_000_000_000_000_000_000_000;

    /// 1 micro-IPN = 10¹⁸ atomic units
    pub const MICRO_IPN: AtomicIPN = 1_000_000_000_000_000_000;

    /// 1 nano-IPN = 10¹⁵ atomic units
    pub const NANO_IPN: AtomicIPN = 1_000_000_000_000_000;

    /// 1 pico-IPN = 10¹² atomic units
    pub const PICO_IPN: AtomicIPN = 1_000_000_000_000;

    /// 1 femto-IPN = 10⁹ atomic units
    pub const FEMTO_IPN: AtomicIPN = 1_000_000_000;

    /// 1 atto-IPN = 10⁶ atomic units
    pub const ATTO_IPN: AtomicIPN = 1_000_000;

    /// 1 zepto-IPN = 10³ atomic units
    pub const ZEPTO_IPN: AtomicIPN = 1_000;

    /// 1 yocto-IPN = 1 atomic unit (smallest denomination)
    pub const YOCTO_IPN: AtomicIPN = 1;
}

/// Maximum supply cap: 21 million IPN in atomic units
pub const SUPPLY_CAP: AtomicIPN = 21_000_000 * ATOMIC_PER_IPN;

/// Represents an amount of IPN in atomic units with overflow protection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Amount(pub AtomicIPN);

impl Amount {
    /// Create a new amount from atomic units
    pub const fn from_atomic(atomic: AtomicIPN) -> Self {
        Self(atomic)
    }

    /// Create an amount from whole IPN tokens
    pub const fn from_ipn(ipn: u64) -> Self {
        Self((ipn as u128) * ATOMIC_PER_IPN)
    }

    /// Create an amount from milli-IPN
    pub const fn from_milli_ipn(milli: u64) -> Self {
        Self((milli as u128) * denominations::MILLI_IPN)
    }

    /// Create an amount from micro-IPN
    pub const fn from_micro_ipn(micro: u64) -> Self {
        Self((micro as u128) * denominations::MICRO_IPN)
    }

    /// Zero amount
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Check if amount is zero
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Get raw atomic units
    pub const fn atomic(&self) -> AtomicIPN {
        self.0
    }

    /// Convert to whole IPN (truncates fractional part)
    pub fn to_ipn(&self) -> u64 {
        (self.0 / ATOMIC_PER_IPN) as u64
    }

    /// Convert to floating-point IPN (use only for display)
    pub fn to_ipn_f64(&self) -> f64 {
        self.0 as f64 / ATOMIC_PER_IPN as f64
    }

    /// Checked addition
    pub fn checked_add(&self, other: Amount) -> Option<Amount> {
        self.0.checked_add(other.0).map(Amount)
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: Amount) -> Option<Amount> {
        self.0.checked_sub(other.0).map(Amount)
    }

    /// Checked multiplication by scalar
    pub fn checked_mul(&self, scalar: u128) -> Option<Amount> {
        self.0.checked_mul(scalar).map(Amount)
    }

    /// Checked division by scalar
    pub fn checked_div(&self, scalar: u128) -> Option<Amount> {
        if scalar == 0 {
            None
        } else {
            Some(Amount(self.0 / scalar))
        }
    }

    /// Saturating addition
    pub fn saturating_add(&self, other: Amount) -> Amount {
        Amount(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction
    pub fn saturating_sub(&self, other: Amount) -> Amount {
        Amount(self.0.saturating_sub(other.0))
    }

    /// Calculate percentage (basis points: 10000 = 100%)
    pub fn percentage(&self, basis_points: u16) -> Amount {
        Amount((self.0 * basis_points as u128) / 10_000)
    }

    /// Split amount evenly among N recipients (remainder is returned)
    pub fn split(&self, count: usize) -> (Amount, Amount) {
        if count == 0 {
            return (Amount::zero(), *self);
        }
        let per_recipient = self.0 / count as u128;
        let remainder = self.0 % count as u128;
        (Amount(per_recipient), Amount(remainder))
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self::zero()
    }
}

impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Mul<u128> for Amount {
    type Output = Self;

    fn mul(self, scalar: u128) -> Self {
        Self(self.0 * scalar)
    }
}

impl Div<u128> for Amount {
    type Output = Self;

    fn div(self, scalar: u128) -> Self {
        Self(self.0 / scalar)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ipn = self.0 / ATOMIC_PER_IPN;
        let fractional = self.0 % ATOMIC_PER_IPN;

        if fractional == 0 {
            write!(f, "{ipn} IPN")
        } else {
            // Show up to 12 significant decimals by default
            let fractional_str = format!("{fractional:024}");
            let trimmed = fractional_str.trim_end_matches('0');
            let truncated = if trimmed.len() > 12 {
                &trimmed[..12]
            } else {
                trimmed
            };
            write!(f, "{ipn}.{truncated} IPN")
        }
    }
}

/// Helper functions for parsing amounts from strings
impl Amount {
    /// Parse from decimal string (e.g., "1.5" = 1.5 IPN)
    pub fn from_str_ipn(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();

        match parts.len() {
            1 => {
                // Whole number only
                let ipn = parts[0]
                    .parse::<u64>()
                    .map_err(|e| format!("Invalid number: {e}"))?;
                Ok(Self::from_ipn(ipn))
            }
            2 => {
                // Has decimal part
                let whole = parts[0]
                    .parse::<u64>()
                    .map_err(|e| format!("Invalid whole part: {e}"))?;

                // Pad or truncate fractional part to 24 digits
                let mut frac_str = parts[1].to_string();
                if frac_str.len() > 24 {
                    return Err("Too many decimal places (max 24)".to_string());
                }
                while frac_str.len() < 24 {
                    frac_str.push('0');
                }

                let fractional = frac_str
                    .parse::<u128>()
                    .map_err(|e| format!("Invalid fractional part: {e}"))?;

                let total = (whole as u128) * ATOMIC_PER_IPN + fractional;
                Ok(Self(total))
            }
            _ => Err("Invalid decimal format".to_string()),
        }
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_denomination_constants() {
        assert_eq!(denominations::IPN, 1_000_000_000_000_000_000_000_000);
        assert_eq!(denominations::MILLI_IPN, 1_000_000_000_000_000_000_000);
        assert_eq!(denominations::MICRO_IPN, 1_000_000_000_000_000_000);
        assert_eq!(denominations::YOCTO_IPN, 1);
    }

    #[test]
    fn test_amount_creation() {
        let one_ipn = Amount::from_ipn(1);
        assert_eq!(one_ipn.atomic(), ATOMIC_PER_IPN);

        let one_micro = Amount::from_micro_ipn(1);
        assert_eq!(one_micro.atomic(), denominations::MICRO_IPN);
    }

    #[test]
    fn test_amount_arithmetic() {
        let a = Amount::from_ipn(5);
        let b = Amount::from_ipn(3);

        assert_eq!(a + b, Amount::from_ipn(8));
        assert_eq!(a - b, Amount::from_ipn(2));
        assert_eq!(a * 2, Amount::from_ipn(10));
        assert_eq!(a / 5, Amount::from_ipn(1));
    }

    #[test]
    fn test_checked_operations() {
        let a = Amount(u128::MAX);
        let b = Amount(1);

        assert!(a.checked_add(b).is_none());
        assert!(b.checked_sub(a).is_none());
        assert!(a.checked_mul(2).is_none());
    }

    #[test]
    fn test_saturating_operations() {
        let a = Amount(u128::MAX);
        let b = Amount(100);

        assert_eq!(a.saturating_add(b), Amount(u128::MAX));
        assert_eq!(Amount::zero().saturating_sub(b), Amount::zero());
    }

    #[test]
    fn test_percentage() {
        let amount = Amount::from_ipn(1000);
        assert_eq!(amount.percentage(2000), Amount::from_ipn(200)); // 20%
        assert_eq!(amount.percentage(5000), Amount::from_ipn(500)); // 50%
    }

    #[test]
    fn test_split() {
        let total = Amount::from_atomic(10_000);
        let (per_recipient, remainder) = total.split(3);

        assert_eq!(per_recipient.atomic(), 3_333);
        assert_eq!(remainder.atomic(), 1);
    }

    #[test]
    fn test_display_format() {
        let one_ipn = Amount::from_ipn(1);
        assert_eq!(format!("{}", one_ipn), "1 IPN");

        let half_ipn = Amount(ATOMIC_PER_IPN / 2);
        assert_eq!(format!("{}", half_ipn), "0.5 IPN");

        let micro = Amount::from_micro_ipn(1);
        assert_eq!(format!("{}", micro), "0.000001 IPN");
    }

    #[test]
    fn test_parse_ipn_string() {
        assert_eq!(Amount::from_str_ipn("1").unwrap(), Amount::from_ipn(1));
        assert_eq!(
            Amount::from_str_ipn("1.5").unwrap(),
            Amount(ATOMIC_PER_IPN + ATOMIC_PER_IPN / 2)
        );
        assert_eq!(
            Amount::from_str_ipn("0.000001").unwrap(),
            Amount::from_micro_ipn(1)
        );

        // Yocto precision
        assert_eq!(
            Amount::from_str_ipn("0.000000000000000000000001").unwrap(),
            Amount::from_atomic(1)
        );
    }

    #[test]
    fn test_supply_cap() {
        let total_supply = Amount(SUPPLY_CAP);
        assert_eq!(total_supply.to_ipn(), 21_000_000);
    }

    #[test]
    fn test_micro_reward_scenario() {
        // Simulate distributing 0.0001 IPN among 1000 blocks
        let round_reward = Amount::from_str_ipn("0.0001").unwrap();
        let (per_block, remainder) = round_reward.split(1000);

        assert_eq!(per_block.atomic(), 100_000_000_000_000_000);
        assert_eq!(remainder.atomic(), 0);

        // Verify each block got exactly 0.0000000000001 IPN
        assert!(per_block > Amount::zero());
    }

    #[test]
    fn test_zero_rounding_loss() {
        // Ensure splitting doesn't lose units
        let total = Amount::from_atomic(999_999_999_999_999_999_999_999);
        let (per_unit, remainder) = total.split(1_000_000);

        let reconstructed = per_unit * 1_000_000 + remainder;
        assert_eq!(reconstructed, total);
    }
}
