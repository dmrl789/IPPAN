//! Domain fees module
//! 
//! Handles fee calculations for domain registration and renewal
//! Implements the 20-year sliding scale fee system

use crate::Result;
use super::DomainConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Premium multipliers for different domain types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PremiumMultiplier {
    /// Standard .ipn domains
    Standard = 1,
    /// IoT domains (.iot)
    IoT = 2,
    /// Premium domains (.ai, .m)
    Premium = 10,
}

impl PremiumMultiplier {
    /// Get multiplier value as u32
    pub fn value(&self) -> u32 {
        *self as u32
    }
    
    /// Determine multiplier from domain name
    pub fn from_domain(domain: &str) -> Self {
        if domain.ends_with(".ai") || domain.ends_with(".m") {
            PremiumMultiplier::Premium
        } else if domain.ends_with(".iot") {
            PremiumMultiplier::IoT
        } else {
            PremiumMultiplier::Standard
        }
    }
}

/// Calculate domain fee for a specific year with premium multiplier
/// Returns fee in micro-IPN units (1e-6 IPN)
/// 
/// Fee schedule:
/// - Year 1: 0.20 IPN × multiplier
/// - Year 2: 0.02 IPN × multiplier  
/// - Year 3-11: 0.01 IPN decreasing by 0.001 each year × multiplier
/// - Year 12+: 0.001 IPN (floor) × multiplier
pub fn domain_fee(year: u32, premium_mult: PremiumMultiplier) -> u64 {
    let base: f64 = if year == 1 {
        0.20
    } else if year == 2 {
        0.02
    } else {
        let decayed = 0.01 - 0.001 * (year as f64 - 3.0);
        if decayed < 0.001 { 0.001 } else { decayed }
    };
    
    (base * premium_mult.value() as f64 * 1_000_000.0).round() as u64
}

/// Calculate total fee for multiple years
pub fn domain_fee_total(start_year: u32, years: u32, premium_mult: PremiumMultiplier) -> u64 {
    let mut total = 0;
    for year in start_year..start_year + years {
        total += domain_fee(year, premium_mult);
    }
    total
}

/// Fee structure for domains (legacy - kept for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainFees {
    /// Standard domain registration fee (per year) - DEPRECATED
    pub standard_registration: u64,
    /// Premium domain registration fee (per year) - DEPRECATED  
    pub premium_registration: u64,
    /// IoT domain registration fee (per year) - DEPRECATED
    pub iot_registration: u64,
    /// AI domain registration fee (per year) - DEPRECATED
    pub ai_registration: u64,
    /// Renewal fee multiplier - DEPRECATED
    pub renewal_multiplier: f64,
    /// Transfer fee
    pub transfer_fee: u64,
    /// Late renewal penalty
    pub late_renewal_penalty: u64,
}

impl Default for DomainFees {
    fn default() -> Self {
        Self {
            standard_registration: 200_000, // 0.20 IPN in micro units
            premium_registration: 2_000_000, // 2.0 IPN in micro units
            iot_registration: 400_000, // 0.40 IPN in micro units
            ai_registration: 2_000_000, // 2.0 IPN in micro units
            renewal_multiplier: 1.0,
            transfer_fee: 5_000, // 0.005 IPN in micro units
            late_renewal_penalty: 2_000_000, // 2.0 IPN in micro units
        }
    }
}

/// Fee record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecord {
    /// Domain name
    pub domain: String,
    /// Fee type
    pub fee_type: FeeType,
    /// Amount paid
    pub amount: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Transaction hash
    pub tx_hash: [u8; 32],
}

/// Fee types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeeType {
    /// Registration fee
    Registration,
    /// Renewal fee
    Renewal,
    /// Premium fee
    Premium,
}

impl DomainFees {
    /// Calculate registration fee for a domain (NEW SLIDING SCALE)
    pub fn calculate_registration_fee(&self, domain: &str, years: u32) -> u64 {
        let premium_mult = PremiumMultiplier::from_domain(domain);
        domain_fee_total(1, years, premium_mult)
    }
    
    /// Calculate renewal fee (NEW SLIDING SCALE)
    pub fn calculate_renewal_fee(&self, domain: &str, current_year: u32, years: u32, is_late: bool) -> u64 {
        let premium_mult = PremiumMultiplier::from_domain(domain);
        let base_fee = domain_fee_total(current_year, years, premium_mult);
        
        if is_late {
            base_fee + self.late_renewal_penalty
        } else {
            base_fee
        }
    }
    
    /// Calculate transfer fee
    pub fn calculate_transfer_fee(&self) -> u64 {
        self.transfer_fee
    }
    
    /// Get fee breakdown for a domain
    pub fn get_fee_breakdown(&self, domain: &str, year: u32) -> FeeBreakdown {
        let premium_mult = PremiumMultiplier::from_domain(domain);
        let yearly_fee = domain_fee(year, premium_mult);
        
        FeeBreakdown {
            domain: domain.to_string(),
            year,
            yearly_fee,
            premium_multiplier: premium_mult.value(),
            total_20_years: domain_fee_total(1, 20, premium_mult),
        }
    }
    
    /// Get 20-year fee schedule for a domain
    pub fn get_20_year_schedule(&self, domain: &str) -> Vec<YearlyFee> {
        let premium_mult = PremiumMultiplier::from_domain(domain);
        let mut schedule = Vec::new();
        
        for year in 1..=20 {
            schedule.push(YearlyFee {
                year,
                fee_micro_ipn: domain_fee(year, premium_mult),
                fee_ipn: domain_fee(year, premium_mult) as f64 / 1_000_000.0,
            });
        }
        
        schedule
    }
}

/// Fee breakdown for a domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    /// Domain name
    pub domain: String,
    /// Current year
    pub year: u32,
    /// Yearly fee in micro-IPN
    pub yearly_fee: u64,
    /// Premium multiplier
    pub premium_multiplier: u32,
    /// Total cost for 20 years in micro-IPN
    pub total_20_years: u64,
}

/// Yearly fee entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyFee {
    /// Year number
    pub year: u32,
    /// Fee in micro-IPN units
    pub fee_micro_ipn: u64,
    /// Fee in IPN
    pub fee_ipn: f64,
}

/// Fee statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStats {
    /// Total revenue
    pub total_revenue: u64,
    /// Registration fees
    pub registration_fees: u64,
    /// Renewal fees
    pub renewal_fees: u64,
    /// Premium fees
    pub premium_fees: u64,
    /// Number of unique domains
    pub unique_domains: u64,
    /// Total number of transactions
    pub total_transactions: u64,
}

impl DomainFees {
    /// Create a new domain fees manager
    pub fn new(_config: &DomainConfig) -> Self {
        Self::default()
    }

    /// Record a fee payment
    pub async fn record_fee(&self, domain: &str, fee_type: FeeType, amount: u64, tx_hash: [u8; 32]) -> Result<()> {
        // Implementation would record to persistent storage
        log::info!("Recorded {} fee for {}: {} micro-IPN", 
            match fee_type {
                FeeType::Registration => "registration",
                FeeType::Renewal => "renewal", 
                FeeType::Premium => "premium",
            },
            domain, amount);
        Ok(())
    }

    /// Get total revenue
    pub async fn get_total_revenue(&self) -> Result<u64> {
        // Implementation would sum from persistent storage
        Ok(0)
    }

    /// Get fee history for a domain
    pub async fn get_domain_fee_history(&self, domain: &str) -> Result<Vec<FeeRecord>> {
        // Implementation would load from persistent storage
        Ok(Vec::new())
    }

    /// Get fee breakdown for a domain
    pub async fn get_domain_fee_breakdown(&self, domain_name: &str) -> Result<FeeBreakdown> {
        let premium_mult = PremiumMultiplier::from_domain(domain_name);
        let yearly_fee = domain_fee(1, premium_mult);

        Ok(FeeBreakdown {
            domain: domain_name.to_string(),
            year: 1,
            yearly_fee,
            premium_multiplier: premium_mult.value(),
            total_20_years: domain_fee_total(1, 20, premium_mult),
        })
    }

    /// Get average fee by type
    pub async fn get_average_fee_by_type(&self) -> Result<HashMap<FeeType, u64>> {
        let fee_history = self.fee_history.read().await;
        let mut fee_sums: HashMap<FeeType, u64> = HashMap::new();
        let mut fee_counts: HashMap<FeeType, u64> = HashMap::new();

        for record in fee_history.iter() {
            *fee_sums.entry(record.fee_type.clone()).or_insert(0) += record.amount;
            *fee_counts.entry(record.fee_type.clone()).or_insert(0) += 1;
        }

        let mut averages = HashMap::new();
        for (fee_type, total) in fee_sums {
            let count = fee_counts.get(&fee_type).unwrap_or(&1);
            averages.insert(fee_type, total / count);
        }

        Ok(averages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_fee_calculation() {
        // Test standard domain fees
        assert_eq!(domain_fee(1, PremiumMultiplier::Standard), 200_000); // 0.20 IPN
        assert_eq!(domain_fee(2, PremiumMultiplier::Standard), 20_000);  // 0.02 IPN
        assert_eq!(domain_fee(3, PremiumMultiplier::Standard), 9_000);   // 0.009 IPN
        assert_eq!(domain_fee(11, PremiumMultiplier::Standard), 1_000);  // 0.001 IPN (floor)
        assert_eq!(domain_fee(20, PremiumMultiplier::Standard), 1_000);  // 0.001 IPN (floor)
        
        // Test premium domain fees (×10)
        assert_eq!(domain_fee(1, PremiumMultiplier::Premium), 2_000_000); // 2.0 IPN
        assert_eq!(domain_fee(2, PremiumMultiplier::Premium), 200_000);   // 0.20 IPN
        assert_eq!(domain_fee(11, PremiumMultiplier::Premium), 10_000);   // 0.01 IPN (floor)
        
        // Test IoT domain fees (×2)
        assert_eq!(domain_fee(1, PremiumMultiplier::IoT), 400_000);      // 0.40 IPN
        assert_eq!(domain_fee(2, PremiumMultiplier::IoT), 40_000);       // 0.04 IPN
        assert_eq!(domain_fee(11, PremiumMultiplier::IoT), 2_000);       // 0.002 IPN (floor)
    }

    #[test]
    fn test_premium_multiplier_detection() {
        assert_eq!(PremiumMultiplier::from_domain("example.ipn"), PremiumMultiplier::Standard);
        assert_eq!(PremiumMultiplier::from_domain("example.iot"), PremiumMultiplier::IoT);
        assert_eq!(PremiumMultiplier::from_domain("example.ai"), PremiumMultiplier::Premium);
        assert_eq!(PremiumMultiplier::from_domain("example.m"), PremiumMultiplier::Premium);
    }

    #[test]
    fn test_total_fee_calculation() {
        // 20 years of standard domain
        let total_standard = domain_fee_total(1, 20, PremiumMultiplier::Standard);
        assert_eq!(total_standard, 266_000); // 0.266 IPN total
        
        // 20 years of premium domain
        let total_premium = domain_fee_total(1, 20, PremiumMultiplier::Premium);
        assert_eq!(total_premium, 2_660_000); // 2.66 IPN total
    }

    #[tokio::test]
    async fn test_fee_calculation() {
        let config = DomainConfig::default();
        let fees = DomainFees::new(&config);

        // Regular domain
        let regular_fee = fees.calculate_registration_fee("example.ipn", 1);
        assert_eq!(regular_fee, 200_000); // 0.20 IPN

        // Premium domain
        let premium_fee = fees.calculate_registration_fee("example.ai", 1);
        assert_eq!(premium_fee, 2_000_000); // 2.0 IPN
    }

    #[tokio::test]
    async fn test_fee_recording() {
        let config = DomainConfig::default();
        let fees = DomainFees::new(&config);

        let domain = "test.ipn";
        let amount = 1_000_000_000;
        let tx_hash = [1u8; 32];

        // Record fee
        fees.record_fee(domain, FeeType::Registration, amount, tx_hash).await.unwrap();

        // Check total revenue
        let total_revenue = fees.get_total_revenue().await.unwrap();
        assert_eq!(total_revenue, amount);

        // Check fee history
        let history = fees.get_domain_fee_history(domain).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].amount, amount);
    }

    #[tokio::test]
    async fn test_premium_tld_detection() {
        let config = DomainConfig::default();
        let fees = DomainFees::new(&config);

        // Test fee breakdown for different domain types
        let standard_breakdown = fees.get_domain_fee_breakdown("example.ipn").await.unwrap();
        assert_eq!(standard_breakdown.premium_multiplier, 1);
        assert_eq!(standard_breakdown.yearly_fee, 200_000); // 0.20 IPN

        let premium_breakdown = fees.get_domain_fee_breakdown("example.ai").await.unwrap();
        assert_eq!(premium_breakdown.premium_multiplier, 10);
        assert_eq!(premium_breakdown.yearly_fee, 2_000_000); // 2.0 IPN

        let iot_breakdown = fees.get_domain_fee_breakdown("example.iot").await.unwrap();
        assert_eq!(iot_breakdown.premium_multiplier, 2);
        assert_eq!(iot_breakdown.yearly_fee, 400_000); // 0.40 IPN
    }
}
