//! Domain renewals module
//! 
//! Handles domain renewal processing and management

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Renewal record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalRecord {
    /// Domain name
    pub domain_name: String,
    /// Renewal timestamp
    pub renewed_at: u64,
    /// New expiration timestamp
    pub expires_at: u64,
    /// Renewal duration in years
    pub duration_years: u64,
    /// Renewal fee paid
    pub fee_paid: u64,
    /// Renewal transaction hash
    pub transaction_hash: Option<[u8; 32]>,
    /// Renewal status
    pub status: RenewalStatus,
}

/// Renewal status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenewalStatus {
    /// Renewal is pending
    Pending,
    /// Renewal is confirmed
    Confirmed,
    /// Renewal failed
    Failed,
    /// Renewal is late
    Late,
}

/// Domain renewal manager
pub struct RenewalManager {
    /// Renewal records
    renewals: Arc<RwLock<HashMap<String, Vec<RenewalRecord>>>>,
    /// Pending renewals
    pending_renewals: Arc<RwLock<HashMap<String, RenewalRecord>>>,
    /// Renewal settings
    settings: RenewalSettings,
}

/// Renewal settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewalSettings {
    /// Grace period in days
    pub grace_period_days: u64,
    /// Auto-renewal enabled
    pub auto_renewal_enabled: bool,
    /// Late renewal penalty percentage
    pub late_renewal_penalty: f64,
    /// Maximum renewal years
    pub max_renewal_years: u64,
    /// Minimum renewal years
    pub min_renewal_years: u64,
}

impl Default for RenewalSettings {
    fn default() -> Self {
        Self {
            grace_period_days: 30,
            auto_renewal_enabled: true,
            late_renewal_penalty: 0.2, // 20%
            max_renewal_years: 10,
            min_renewal_years: 1,
        }
    }
}

impl RenewalManager {
    /// Create a new renewal manager
    pub fn new(settings: RenewalSettings) -> Self {
        Self {
            renewals: Arc::new(RwLock::new(HashMap::new())),
            pending_renewals: Arc::new(RwLock::new(HashMap::new())),
            settings,
        }
    }
    
    /// Request domain renewal
    pub async fn request_renewal(
        &self,
        domain_name: String,
        duration_years: u64,
        fee_paid: u64,
    ) -> Result<RenewalRecord> {
        // Validate renewal duration
        if duration_years < self.settings.min_renewal_years {
            return Err(crate::error::IppanError::Domain(
                format!("Renewal duration must be at least {} years", self.settings.min_renewal_years)
            ));
        }
        
        if duration_years > self.settings.max_renewal_years {
            return Err(crate::error::IppanError::Domain(
                format!("Renewal duration cannot exceed {} years", self.settings.max_renewal_years)
            ));
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expires_at = now + (duration_years * 365 * 24 * 60 * 60);
        
        let renewal = RenewalRecord {
            domain_name: domain_name.clone(),
            renewed_at: now,
            expires_at,
            duration_years,
            fee_paid,
            transaction_hash: None,
            status: RenewalStatus::Pending,
        };
        
        // Add to pending renewals
        {
            let mut pending = self.pending_renewals.write().await;
            pending.insert(domain_name.clone(), renewal.clone());
        }
        
        Ok(renewal)
    }
    
    /// Confirm renewal
    pub async fn confirm_renewal(&self, domain_name: &str, transaction_hash: [u8; 32]) -> Result<()> {
        let mut pending = self.pending_renewals.write().await;
        
        if let Some(mut renewal) = pending.remove(domain_name) {
            renewal.status = RenewalStatus::Confirmed;
            renewal.transaction_hash = Some(transaction_hash);
            
            // Add to renewal history
            {
                let mut renewals = self.renewals.write().await;
                renewals.entry(domain_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(renewal);
            }
            
            Ok(())
        } else {
            Err(crate::error::IppanError::Domain(
                format!("No pending renewal found for domain {}", domain_name)
            ))
        }
    }
    
    /// Process late renewal
    pub async fn process_late_renewal(
        &self,
        domain_name: String,
        duration_years: u64,
        fee_paid: u64,
    ) -> Result<RenewalRecord> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expires_at = now + (duration_years * 365 * 24 * 60 * 60);
        
        let renewal = RenewalRecord {
            domain_name: domain_name.clone(),
            renewed_at: now,
            expires_at,
            duration_years,
            fee_paid,
            transaction_hash: None,
            status: RenewalStatus::Late,
        };
        
        // Add to renewal history
        {
            let mut renewals = self.renewals.write().await;
            renewals.entry(domain_name)
                .or_insert_with(Vec::new)
                .push(renewal.clone());
        }
        
        Ok(renewal)
    }
    
    /// Get renewal history for domain
    pub async fn get_renewal_history(&self, domain_name: &str) -> Vec<RenewalRecord> {
        let renewals = self.renewals.read().await;
        renewals.get(domain_name).cloned().unwrap_or_default()
    }
    
    /// Get pending renewals
    pub async fn get_pending_renewals(&self) -> Vec<RenewalRecord> {
        let pending = self.pending_renewals.read().await;
        pending.values().cloned().collect()
    }
    
    /// Check if domain has pending renewal
    pub async fn has_pending_renewal(&self, domain_name: &str) -> bool {
        let pending = self.pending_renewals.read().await;
        pending.contains_key(domain_name)
    }
    
    /// Get renewal settings
    pub fn get_settings(&self) -> &RenewalSettings {
        &self.settings
    }
    
    /// Update renewal settings
    pub fn update_settings(&mut self, settings: RenewalSettings) {
        self.settings = settings;
    }
    
    /// Calculate late renewal penalty
    pub fn calculate_late_penalty(&self, base_fee: u64) -> u64 {
        (base_fee as f64 * self.settings.late_renewal_penalty) as u64
    }
    
    /// Get grace period end time
    pub fn get_grace_period_end(&self, expiration_time: u64) -> u64 {
        expiration_time + (self.settings.grace_period_days * 24 * 60 * 60)
    }
    
    /// Check if domain is in grace period
    pub fn is_in_grace_period(&self, expiration_time: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let grace_period_end = self.get_grace_period_end(expiration_time);
        now <= grace_period_end
    }
    
    /// Get all renewal records
    pub async fn get_all_renewals(&self) -> HashMap<String, Vec<RenewalRecord>> {
        self.renewals.read().await.clone()
    }
    
    /// Get renewal count for domain
    pub async fn get_renewal_count(&self, domain_name: &str) -> usize {
        let renewals = self.renewals.read().await;
        renewals.get(domain_name).map(|r| r.len()).unwrap_or(0)
    }
}
