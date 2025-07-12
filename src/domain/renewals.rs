//! Domain renewals module
//! 
//! Handles domain renewal processing and management

use crate::{
    wallet::WalletManager,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Renewal manager for domain renewals and expiration tracking
pub struct RenewalManager {
    /// Renewal records by domain
    renewals: HashMap<String, Vec<RenewalRecord>>,
    /// Expiration tracking
    expiration_tracker: HashMap<String, SystemTime>,
    /// Wallet for fee collection
    wallet: Arc<RwLock<WalletManager>>,
    /// Total renewal revenue
    total_revenue: u64,
}

impl RenewalManager {
    pub fn new(wallet: Arc<RwLock<WalletManager>>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            renewals: HashMap::new(),
            expiration_tracker: HashMap::new(),
            wallet,
            total_revenue: 0,
        })
    }

    /// Start the renewal manager
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting renewal manager...");
        
        // Start background expiration monitoring
        self.start_expiration_monitoring().await?;
        
        log::info!("Renewal manager started");
        Ok(())
    }

    /// Stop the renewal manager
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping renewal manager...");
        
        // Stop background monitoring
        self.stop_expiration_monitoring().await?;
        
        log::info!("Renewal manager stopped");
        Ok(())
    }

    /// Renew a domain
    pub async fn renew_domain(&mut self, handle: &str, duration_years: u32) -> Result<crate::domain::DomainRenewal, crate::domain::DomainError> {
        // Calculate renewal fee
        let renewal_fee = self.calculate_renewal_fee(duration_years);
        
        // Create renewal record
        let renewal_date = SystemTime::now();
        let new_expiry_date = renewal_date + Duration::from_secs(duration_years as u64 * 365 * 24 * 60 * 60);
        
        let renewal_record = RenewalRecord {
            renewal_date,
            duration_years,
            fee_paid: renewal_fee,
            new_expiry_date,
        };
        
        // Add to renewal history
        self.renewals.entry(handle.to_string())
            .or_insert_with(Vec::new)
            .push(renewal_record.clone());
        
        // Update expiration tracker
        self.expiration_tracker.insert(handle.to_string(), new_expiry_date);
        
        // Update revenue
        self.total_revenue += renewal_fee;
        
        // Add fee to Global Fund (wallet)
        let mut wallet = self.wallet.write().await;
        wallet.add_domain_fee(renewal_fee).await?;
        
        Ok(crate::domain::DomainRenewal {
            handle: handle.to_string(),
            renewal_date,
            new_expiry_date,
            duration_years,
            fee_paid: renewal_fee,
        })
    }

    /// Get renewal history for a domain
    pub fn get_renewal_history(&self, handle: &str) -> Vec<RenewalRecord> {
        self.renewals.get(handle)
            .cloned()
            .unwrap_or_default()
    }

    /// Get domains expiring within days
    pub fn get_expiring_domains(&self, days: u32) -> Vec<crate::domain::DomainInfo> {
        let now = SystemTime::now();
        let threshold = now + Duration::from_secs(days as u64 * 24 * 60 * 60);
        
        self.expiration_tracker.iter()
            .filter(|(_, expiry_date)| **expiry_date > now && **expiry_date <= threshold)
            .map(|(handle, expiry_date)| {
                // This would need to be populated with actual domain info
                // For now, create a placeholder
                crate::domain::DomainInfo {
                    handle: handle.clone(),
                    owner_address: "".to_string(),
                    registration_date: SystemTime::now(),
                    expiry_date: *expiry_date,
                    is_active: true,
                    transfer_count: 0,
                    renewal_count: 0,
                }
            })
            .collect()
    }

    /// Get expired domains
    pub fn get_expired_domains(&self) -> Vec<String> {
        let now = SystemTime::now();
        self.expiration_tracker.iter()
            .filter(|(_, expiry_date)| **expiry_date <= now)
            .map(|(handle, _)| handle.clone())
            .collect()
    }

    /// Get total renewal revenue
    pub fn get_total_revenue(&self) -> u64 {
        self.total_revenue
    }

    /// Get renewal statistics
    pub fn get_renewal_stats(&self) -> RenewalStats {
        let total_renewals = self.renewals.values().map(|v| v.len()).sum();
        let expiring_soon = self.get_expiring_domains(30).len();
        let expired = self.get_expired_domains().len();
        
        RenewalStats {
            total_renewals,
            expiring_soon,
            expired,
            total_revenue: self.total_revenue,
        }
    }

    /// Calculate renewal fee
    fn calculate_renewal_fee(&self, duration_years: u32) -> u64 {
        let base_fee = 500_000; // 0.5 IPN in smallest units
        base_fee * duration_years as u64
    }

    /// Start background expiration monitoring
    async fn start_expiration_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let expiration_tracker = Arc::new(RwLock::new(self.expiration_tracker.clone()));
        
        tokio::spawn(async move {
            loop {
                // Check for expired domains every hour
                tokio::time::sleep(Duration::from_secs(3600)).await;
                
                let tracker = expiration_tracker.read().await;
                let now = SystemTime::now();
                
                let expired: Vec<_> = tracker.iter()
                    .filter(|(_, expiry_date)| **expiry_date <= now)
                    .map(|(handle, _)| handle.clone())
                    .collect();
                
                if !expired.is_empty() {
                    log::warn!("Found {} expired domains: {:?}", expired.len(), expired);
                    // In a real implementation, this would trigger domain expiration logic
                }
            }
        });
        
        Ok(())
    }

    /// Stop background expiration monitoring
    async fn stop_expiration_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would stop the background task
        Ok(())
    }

    /// Add domain to expiration tracking
    pub fn add_domain_to_tracking(&mut self, handle: &str, expiry_date: SystemTime) {
        self.expiration_tracker.insert(handle.to_string(), expiry_date);
    }

    /// Remove domain from expiration tracking
    pub fn remove_domain_from_tracking(&mut self, handle: &str) {
        self.expiration_tracker.remove(handle);
    }

    /// Update domain expiry date
    pub fn update_domain_expiry(&mut self, handle: &str, new_expiry: SystemTime) {
        self.expiration_tracker.insert(handle.to_string(), new_expiry);
    }
}

/// Renewal record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalRecord {
    pub renewal_date: SystemTime,
    pub duration_years: u32,
    pub fee_paid: u64,
    pub new_expiry_date: SystemTime,
}

/// Renewal statistics
#[derive(Debug, Serialize)]
pub struct RenewalStats {
    pub total_renewals: usize,
    pub expiring_soon: usize,
    pub expired: usize,
    pub total_revenue: u64,
}
