//! IPPAN Atomic Units Module
//!
//! Implements ultra-fine divisibility with 24-decimal precision for IPN tokens.
//! Supports HashTimer-anchored micropayments and DAG-Fair emission with atomic precision.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// IPN is stored as fixed-point integer with 24 decimal places.
/// 1 IPN = 10^24 atomic units.
pub type AtomicIPN = u128;

/// Number of decimal places for IPN precision
pub const IPN_DECIMALS: u32 = 24;

/// Conversion factor: 1 IPN = 10^24 atomic units
pub const ATOMIC_PER_IPN: AtomicIPN = 10u128.pow(IPN_DECIMALS);

/// IPN fractional unit denominations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IPNUnit {
    /// 1 IPN = 1.0
    IPN,
    /// 1 mIPN = 0.001 IPN
    MilliIPN,
    /// 1 µIPN = 0.000001 IPN
    MicroIPN,
    /// 1 aIPN = 10^-18 IPN
    AttoIPN,
    /// 1 zIPN = 10^-21 IPN
    ZeptoIPN,
    /// 1 yIPN = 10^-24 IPN (smallest unit)
    YoctoIPN,
}

impl IPNUnit {
    /// Get the conversion factor from this unit to atomic units
    pub fn to_atomic_factor(self) -> u128 {
        match self {
            IPNUnit::IPN => ATOMIC_PER_IPN,
            IPNUnit::MilliIPN => ATOMIC_PER_IPN / 1_000,
            IPNUnit::MicroIPN => ATOMIC_PER_IPN / 1_000_000,
            IPNUnit::AttoIPN => ATOMIC_PER_IPN / 1_000_000_000_000_000_000,
            IPNUnit::ZeptoIPN => ATOMIC_PER_IPN / 1_000_000_000_000_000_000_000,
            IPNUnit::YoctoIPN => 1, // 1 atomic unit
        }
    }

    /// Get the symbol for this unit
    pub fn symbol(self) -> &'static str {
        match self {
            IPNUnit::IPN => "IPN",
            IPNUnit::MilliIPN => "mIPN",
            IPNUnit::MicroIPN => "µIPN",
            IPNUnit::AttoIPN => "aIPN",
            IPNUnit::ZeptoIPN => "zIPN",
            IPNUnit::YoctoIPN => "yIPN",
        }
    }

    /// Get the name for this unit
    pub fn name(self) -> &'static str {
        match self {
            IPNUnit::IPN => "IPN",
            IPNUnit::MilliIPN => "milli-IPN",
            IPNUnit::MicroIPN => "micro-IPN",
            IPNUnit::AttoIPN => "atto-IPN",
            IPNUnit::ZeptoIPN => "zepto-IPN",
            IPNUnit::YoctoIPN => "yocto-IPN",
        }
    }
}

/// Represents an amount of IPN with atomic precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IPNAmount {
    /// Amount in atomic units (10^-24 IPN precision)
    pub atomic: AtomicIPN,
}

impl IPNAmount {
    /// Create a new IPN amount from atomic units
    pub fn from_atomic(atomic: AtomicIPN) -> Self {
        Self { atomic }
    }

    /// Create a new IPN amount from a specific unit
    pub fn from_unit(amount: u128, unit: IPNUnit) -> Self {
        Self {
            atomic: amount.saturating_mul(unit.to_atomic_factor()),
        }
    }

    /// Create 1 IPN
    pub fn one_ipn() -> Self {
        Self::from_unit(1, IPNUnit::IPN)
    }

    /// Create 1 yocto-IPN (smallest unit)
    pub fn one_yocto() -> Self {
        Self::from_atomic(1)
    }

    /// Get the amount in atomic units
    pub fn atomic(&self) -> AtomicIPN {
        self.atomic
    }

    /// Convert to a specific unit (with rounding)
    pub fn to_unit(&self, unit: IPNUnit) -> u128 {
        self.atomic / unit.to_atomic_factor()
    }

    /// Convert to IPN (with fractional part)
    pub fn to_ipn_f64(&self) -> f64 {
        self.atomic as f64 / ATOMIC_PER_IPN as f64
    }

    /// Check if this amount is zero
    pub fn is_zero(&self) -> bool {
        self.atomic == 0
    }

    /// Check if this amount is positive
    pub fn is_positive(&self) -> bool {
        self.atomic > 0
    }

    /// Get the fractional part in yocto-IPN
    pub fn fractional_yocto(&self) -> u64 {
        (self.atomic % ATOMIC_PER_IPN) as u64
    }
}

impl Add for IPNAmount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            atomic: self.atomic.saturating_add(other.atomic),
        }
    }
}

impl Sub for IPNAmount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            atomic: self.atomic.saturating_sub(other.atomic),
        }
    }
}

impl Mul<u128> for IPNAmount {
    type Output = Self;

    fn mul(self, rhs: u128) -> Self {
        Self {
            atomic: self.atomic.saturating_mul(rhs),
        }
    }
}

impl Div<u128> for IPNAmount {
    type Output = Self;

    fn div(self, rhs: u128) -> Self {
        Self {
            atomic: self.atomic / rhs,
        }
    }
}

impl fmt::Display for IPNAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.atomic == 0 {
            return write!(f, "0 IPN");
        }

        let ipn_part = self.atomic / ATOMIC_PER_IPN;
        let fractional_part = self.atomic % ATOMIC_PER_IPN;

        if ipn_part > 0 {
            if fractional_part == 0 {
                write!(f, "{} IPN", ipn_part)
            } else {
                // Show up to 8 decimal places for readability
                let fractional_display = fractional_part / (ATOMIC_PER_IPN / 100_000_000);
                write!(f, "{}.{:08} IPN", ipn_part, fractional_display)
            }
        } else {
            // Show in smallest appropriate unit
            if fractional_part >= IPNUnit::MicroIPN.to_atomic_factor() {
                let micro = fractional_part / IPNUnit::MicroIPN.to_atomic_factor();
                write!(f, "{} µIPN", micro)
            } else if fractional_part >= IPNUnit::AttoIPN.to_atomic_factor() {
                let atto = fractional_part / IPNUnit::AttoIPN.to_atomic_factor();
                write!(f, "{} aIPN", atto)
            } else if fractional_part >= IPNUnit::ZeptoIPN.to_atomic_factor() {
                let zepto = fractional_part / IPNUnit::ZeptoIPN.to_atomic_factor();
                write!(f, "{} zIPN", zepto)
            } else {
                write!(f, "{} yIPN", fractional_part)
            }
        }
    }
}

/// Convert IPN amount to atomic units for fee calculations
impl From<IPNAmount> for AtomicIPN {
    fn from(amount: IPNAmount) -> Self {
        amount.atomic
    }
}

/// Convert atomic units to IPN amount
impl From<AtomicIPN> for IPNAmount {
    fn from(atomic: AtomicIPN) -> Self {
        Self::from_atomic(atomic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversions() {
        // Test 1 IPN = 10^24 atomic units
        let one_ipn = IPNAmount::from_unit(1, IPNUnit::IPN);
        assert_eq!(one_ipn.atomic(), ATOMIC_PER_IPN);

        // Test 1 mIPN = 10^21 atomic units
        let one_milli = IPNAmount::from_unit(1, IPNUnit::MilliIPN);
        assert_eq!(one_milli.atomic(), ATOMIC_PER_IPN / 1_000);

        // Test 1 µIPN = 10^18 atomic units
        let one_micro = IPNAmount::from_unit(1, IPNUnit::MicroIPN);
        assert_eq!(one_micro.atomic(), ATOMIC_PER_IPN / 1_000_000);

        // Test 1 yIPN = 1 atomic unit
        let one_yocto = IPNAmount::from_unit(1, IPNUnit::YoctoIPN);
        assert_eq!(one_yocto.atomic(), 1);
    }

    #[test]
    fn test_arithmetic() {
        let a = IPNAmount::from_unit(1, IPNUnit::IPN);
        let b = IPNAmount::from_unit(500, IPNUnit::MilliIPN);
        let sum = a + b;
        assert_eq!(sum.atomic(), ATOMIC_PER_IPN + (ATOMIC_PER_IPN / 2));

        let diff = a - b;
        assert_eq!(diff.atomic(), ATOMIC_PER_IPN / 2);
    }

    #[test]
    fn test_display() {
        let one_ipn = IPNAmount::one_ipn();
        let one_ipn_str = format!("{}", one_ipn);
        assert!(one_ipn_str.contains("1 IPN"));

        let half_ipn = IPNAmount::from_unit(500, IPNUnit::MilliIPN);
        let half_ipn_str = format!("{}", half_ipn);
        // 500 mIPN = 500,000 µIPN (since 1 mIPN = 1000 µIPN)
        assert!(half_ipn_str.contains("500000 µIPN"));

        let one_yocto = IPNAmount::one_yocto();
        let one_yocto_str = format!("{}", one_yocto);
        assert!(one_yocto_str.contains("1 yIPN"));
    }

    #[test]
    fn test_validator_reward_example() {
        // Example from the documentation:
        // Round reward R(t) = 0.0001 IPN = 10^20 atomic units
        let round_reward = IPNAmount::from_unit(100, IPNUnit::MicroIPN);
        assert_eq!(round_reward.atomic(), 100 * IPNUnit::MicroIPN.to_atomic_factor());

        // Blocks in round B_r = 1,000
        let blocks_in_round = 1000;
        let per_block = round_reward / blocks_in_round;
        
        // Per block = 10^17 atomic units = 0.0000000000001 IPN
        let expected_per_block = IPNAmount::from_atomic(100 * IPNUnit::MicroIPN.to_atomic_factor() / 1000);
        assert_eq!(per_block.atomic(), expected_per_block.atomic());
    }
}