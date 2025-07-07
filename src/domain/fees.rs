//! Domain fees module
//! 
//! Handles fee calculations for domain registration and renewal

use crate::Result;
use super::DomainConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Fee structure for domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainFees {
    /// Standard domain registration fee (per year)
    pub standard_registration: u64,
    /// Premium domain registration fee (per year)
    pub premium_registration: u64,
    /// IoT domain registration fee (per year)
    pub iot_registration: u64,
    /// AI domain registration fee (per year)
    pub ai_registration: u64,
    /// Renewal fee multiplier
    pub renewal_multiplier: f64,
    /// Transfer fee
    pub transfer_fee: u64,
    /// Late renewal penalty
    pub late_renewal_penalty: u64,
}

impl Default for DomainFees {
    fn default() -> Self {
        Self {
            standard_registration: 1_000_000, // 0.01 IPN
            premium_registration: 10_000_000, // 0.1 IPN
            iot_registration: 5_000_000, // 0.05 IPN
            ai_registration: 15_000_000, // 0.15 IPN
            renewal_multiplier: 1.0,
            transfer_fee: 500_000, // 0.005 IPN
            late_renewal_penalty: 2_000_000, // 0.02 IPN
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
    /// Calculate registration fee for a domain
    pub fn calculate_registration_fee(&self, domain_type: &str, years: u64) -> u64 {
        let base_fee = match domain_type {
            "standard" => self.standard_registration,
            "premium" => self.premium_registration,
            "iot" => self.iot_registration,
            "ai" => self.ai_registration,
            _ => self.standard_registration,
        };
        
        base_fee * years
    }
    
    /// Calculate renewal fee
    pub fn calculate_renewal_fee(&self, domain_type: &str, years: u64, is_late: bool) -> u64 {
        let base_fee = self.calculate_registration_fee(domain_type, years);
        let renewal_fee = (base_fee as f64 * self.renewal_multiplier) as u64;
        
        if is_late {
            renewal_fee + self.late_renewal_penalty
        } else {
            renewal_fee
        }
    }
    
    /// Calculate transfer fee
    pub fn calculate_transfer_fee(&self) -> u64 {
        self.transfer_fee
    }
    
    /// Get fee for domain type
    pub fn get_fee_for_type(&self, domain_type: &str) -> u64 {
        match domain_type {
            "standard" => self.standard_registration,
            "premium" => self.premium_registration,
            "iot" => self.iot_registration,
            "ai" => self.ai_registration,
            _ => self.standard_registration,
        }
    }
}

impl DomainFees {
    /// Create a new domain fees manager
    pub fn new(config: &DomainConfig) -> Self {
        Self {
            config: config.clone(),
            fee_history: RwLock::new(Vec::new()),
            total_revenue: RwLock::new(0),
        }
    }

    /// Record a fee payment
    pub async fn record_fee(&self, domain: &str, fee_type: FeeType, amount: u64, tx_hash: [u8; 32]) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let fee_record = FeeRecord {
            domain: domain.to_string(),
            fee_type,
            amount,
            timestamp,
            tx_hash,
        };

        // Add to fee history
        {
            let mut fee_history = self.fee_history.write().await;
            fee_history.push(fee_record);
        }

        // Update total revenue
        {
            let mut total_revenue = self.total_revenue.write().await;
            *total_revenue += amount;
        }

        Ok(())
    }

    /// Get total revenue
    pub async fn get_total_revenue(&self) -> Result<u64> {
        Ok(*self.total_revenue.read().await)
    }

    /// Get fee history for a domain
    pub async fn get_domain_fee_history(&self, domain: &str) -> Result<Vec<FeeRecord>> {
        let fee_history = self.fee_history.read().await;
        let mut results = fee_history.iter()
            .filter(|record| record.domain == domain)
            .cloned()
            .collect::<Vec<_>>();

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(results)
    }

    /// Get fee statistics
    pub async fn get_fee_stats(&self) -> Result<FeeStats> {
        let fee_history = self.fee_history.read().await;
        let total_revenue = *self.total_revenue.read().await;

        let mut registration_fees = 0;
        let mut renewal_fees = 0;
        let mut premium_fees = 0;
        let mut domain_count = std::collections::HashSet::new();

        for record in fee_history.iter() {
            domain_count.insert(record.domain.clone());
            
            match record.fee_type {
                FeeType::Registration => registration_fees += record.amount,
                FeeType::Renewal => renewal_fees += record.amount,
                FeeType::Premium => premium_fees += record.amount,
            }
        }

        Ok(FeeStats {
            total_revenue,
            registration_fees,
            renewal_fees,
            premium_fees,
            unique_domains: domain_count.len() as u64,
            total_transactions: fee_history.len() as u64,
        })
    }

    /// Get revenue for a time period
    pub async fn get_revenue_for_period(&self, start_time: u64, end_time: u64) -> Result<u64> {
        let fee_history = self.fee_history.read().await;
        let mut revenue = 0;

        for record in fee_history.iter() {
            if record.timestamp >= start_time && record.timestamp <= end_time {
                revenue += record.amount;
            }
        }

        Ok(revenue)
    }

    /// Check if domain has a premium TLD
    fn is_premium_tld(&self, domain_name: &str) -> bool {
        if let Some(tld) = domain_name.split('.').nth(1) {
            self.config.premium_tlds.contains(&tld.to_string())
        } else {
            false
        }
    }

    /// Get fee breakdown for a domain
    pub async fn get_domain_fee_breakdown(&self, domain_name: &str) -> Result<FeeBreakdown> {
        let registration_fee = self.calculate_registration_fee("standard", 1).unwrap();
        let renewal_fee = self.calculate_renewal_fee("standard", 1, false);
        let is_premium = self.is_premium_tld(domain_name);

        Ok(FeeBreakdown {
            domain: domain_name.to_string(),
            registration_fee,
            renewal_fee,
            is_premium,
            premium_multiplier: if is_premium { self.renewal_multiplier } else { 1.0 },
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

/// Fee breakdown for a domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    /// Domain name
    pub domain: String,
    /// Registration fee
    pub registration_fee: u64,
    /// Renewal fee
    pub renewal_fee: u64,
    /// Whether it's a premium domain
    pub is_premium: bool,
    /// Premium multiplier
    pub premium_multiplier: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fee_calculation() {
        let config = DomainConfig::default();
        let fees = DomainFees::new(&config);

        // Regular domain
        let regular_fee = fees.calculate_registration_fee("standard", 1);
        assert_eq!(regular_fee, 1_000_000);

        // Premium domain
        let premium_fee = fees.calculate_registration_fee("premium", 1);
        assert_eq!(premium_fee, 10_000_000);
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

        // Test premium TLDs
        assert!(fees.is_premium_tld("alice.m"));
        assert!(fees.is_premium_tld("bob.cyborg"));
        assert!(fees.is_premium_tld("charlie.humanoid"));

        // Test regular TLD
        assert!(!fees.is_premium_tld("alice.ipn"));
        assert!(!fees.is_premium_tld("bob.com"));
    }
}
