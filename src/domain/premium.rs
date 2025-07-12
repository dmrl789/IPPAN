//! Premium domain module
//! 
//! Handles premium domain types and features

use crate::{
    wallet::WalletManager,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Premium TLD manager for handling premium top-level domains
pub struct PremiumTldManager {
    /// Registered premium TLDs
    premium_tlds: HashMap<String, PremiumTldRecord>,
    /// TLD ownership by address
    tld_ownership: HashMap<String, Vec<String>>,
    /// Wallet for fee collection
    wallet: Arc<RwLock<WalletManager>>,
    /// Total premium TLD revenue
    total_revenue: u64,
}

impl PremiumTldManager {
    pub fn new(wallet: Arc<RwLock<WalletManager>>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            premium_tlds: HashMap::new(),
            tld_ownership: HashMap::new(),
            wallet,
            total_revenue: 0,
        })
    }

    /// Start the premium TLD manager
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting premium TLD manager...");
        
        // Load existing premium TLDs from storage
        self.load_premium_tlds().await?;
        
        log::info!("Premium TLD manager started");
        Ok(())
    }

    /// Stop the premium TLD manager
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping premium TLD manager...");
        
        // Save premium TLDs to storage
        self.save_premium_tlds().await?;
        
        log::info!("Premium TLD manager stopped");
        Ok(())
    }

    /// Register a premium TLD
    pub async fn register_tld(&mut self, tld: &str, owner_address: &str) -> Result<crate::domain::PremiumTldRegistration, crate::domain::DomainError> {
        // Check if TLD is already registered
        if self.premium_tlds.contains_key(tld) {
            return Err(crate::domain::DomainError::TldAlreadyRegistered { tld: tld.to_string() });
        }
        
        // Validate TLD format
        self.validate_tld_format(tld)?;
        
        // Create premium TLD record
        let registration_date = SystemTime::now();
        let premium_fee = 10_000_000; // 10 IPN in smallest units
        
        let tld_record = PremiumTldRecord {
            tld: tld.to_string(),
            owner_address: owner_address.to_string(),
            registration_date,
            is_active: true,
            annual_fee: premium_fee,
            features: self.get_default_features(tld),
        };
        
        // Add to premium TLDs
        self.premium_tlds.insert(tld.to_string(), tld_record.clone());
        
        // Update ownership index
        self.tld_ownership.entry(owner_address.to_string())
            .or_insert_with(Vec::new)
            .push(tld.to_string());
        
        // Update revenue
        self.total_revenue += premium_fee;
        
        // Add fee to Global Fund (wallet)
        let mut wallet = self.wallet.write().await;
        wallet.add_domain_fee(premium_fee).await?;
        
        Ok(crate::domain::PremiumTldRegistration {
            tld: tld.to_string(),
            owner_address: owner_address.to_string(),
            registration_date,
            fee_paid: premium_fee,
        })
    }

    /// Transfer premium TLD ownership
    pub async fn transfer_tld(&mut self, tld: &str, new_owner: &str) -> Result<crate::domain::DomainTransfer, crate::domain::DomainError> {
        let tld_record = self.premium_tlds.get_mut(tld)
            .ok_or_else(|| crate::domain::DomainError::DomainNotFound { handle: tld.to_string() })?;
        
        let previous_owner = tld_record.owner_address.clone();
        
        // Remove from previous owner's list
        if let Some(tlds) = self.tld_ownership.get_mut(&previous_owner) {
            tlds.retain(|t| t != tld);
        }
        
        // Update TLD record
        tld_record.owner_address = new_owner.to_string();
        
        // Add to new owner's list
        self.tld_ownership.entry(new_owner.to_string())
            .or_insert_with(Vec::new)
            .push(tld.to_string());
        
        Ok(crate::domain::DomainTransfer {
            handle: tld.to_string(),
            previous_owner,
            new_owner: new_owner.to_string(),
            transfer_date: SystemTime::now(),
        })
    }

    /// Get premium TLD information
    pub fn get_tld_info(&self, tld: &str) -> Option<PremiumTldInfo> {
        self.premium_tlds.get(tld).map(|record| PremiumTldInfo {
            tld: record.tld.clone(),
            owner_address: record.owner_address.clone(),
            registration_date: record.registration_date,
            is_active: record.is_active,
            annual_fee: record.annual_fee,
            features: record.features.clone(),
        })
    }

    /// Check if premium TLD is available
    pub fn is_tld_available(&self, tld: &str) -> bool {
        !self.premium_tlds.contains_key(tld)
    }

    /// Get premium TLDs owned by an address
    pub fn get_tlds_by_owner(&self, owner_address: &str) -> Vec<PremiumTldInfo> {
        self.tld_ownership.get(owner_address)
            .map(|tlds| {
                tlds.iter()
                    .filter_map(|tld| self.get_tld_info(tld))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all premium TLDs
    pub fn get_all_premium_tlds(&self) -> Vec<PremiumTldInfo> {
        self.premium_tlds.values()
            .map(|record| PremiumTldInfo {
                tld: record.tld.clone(),
                owner_address: record.owner_address.clone(),
                registration_date: record.registration_date,
                is_active: record.is_active,
                annual_fee: record.annual_fee,
                features: record.features.clone(),
            })
            .collect()
    }

    /// Get total number of premium TLDs
    pub fn get_total_tlds(&self) -> u64 {
        self.premium_tlds.len() as u64
    }

    /// Get total revenue from premium TLDs
    pub fn get_total_revenue(&self) -> u64 {
        self.total_revenue
    }

    /// Get premium TLD statistics
    pub fn get_tld_stats(&self) -> PremiumTldStats {
        let active_tlds = self.premium_tlds.values()
            .filter(|record| record.is_active)
            .count();
        
        let total_owners = self.tld_ownership.len();
        
        PremiumTldStats {
            total_tlds: self.premium_tlds.len(),
            active_tlds,
            total_owners,
            total_revenue: self.total_revenue,
        }
    }

    /// Validate TLD format
    fn validate_tld_format(&self, tld: &str) -> Result<(), crate::domain::DomainError> {
        if tld.is_empty() || tld.len() > 10 {
            return Err(crate::domain::DomainError::InvalidTldFormat { tld: tld.to_string() });
        }
        
        // Check for valid characters (alphanumeric, hyphens)
        if !tld.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(crate::domain::DomainError::InvalidTldFormat { tld: tld.to_string() });
        }
        
        // Check for reserved TLDs
        let reserved_tlds = ["ipn", "iot", "ai", "m", "cyborg", "humanoid"];
        if reserved_tlds.contains(&tld) {
            return Err(crate::domain::DomainError::InvalidTldFormat { 
                tld: format!("{} is a reserved TLD", tld) 
            });
        }
        
        Ok(())
    }

    /// Get default features for a premium TLD
    fn get_default_features(&self, tld: &str) -> Vec<String> {
        match tld {
            "m" => vec!["mobile_optimized".to_string(), "short_names".to_string()],
            "cyborg" => vec!["ai_integration".to_string(), "enhanced_security".to_string()],
            "humanoid" => vec!["human_centric".to_string(), "accessibility".to_string()],
            _ => vec!["premium_support".to_string(), "priority_resolution".to_string()],
        }
    }

    /// Load premium TLDs from storage
    async fn load_premium_tlds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would load from persistent storage
        // For now, start with empty registry
        log::info!("Loading premium TLDs from storage...");
        Ok(())
    }

    /// Save premium TLDs to storage
    async fn save_premium_tlds(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would save to persistent storage
        log::info!("Saving {} premium TLDs to storage...", self.premium_tlds.len());
        Ok(())
    }
}

/// Premium TLD record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumTldRecord {
    pub tld: String,
    pub owner_address: String,
    pub registration_date: SystemTime,
    pub is_active: bool,
    pub annual_fee: u64,
    pub features: Vec<String>,
}

/// Premium TLD information
#[derive(Debug, Serialize)]
pub struct PremiumTldInfo {
    pub tld: String,
    pub owner_address: String,
    pub registration_date: SystemTime,
    pub is_active: bool,
    pub annual_fee: u64,
    pub features: Vec<String>,
}

/// Premium TLD statistics
#[derive(Debug, Serialize)]
pub struct PremiumTldStats {
    pub total_tlds: usize,
    pub active_tlds: usize,
    pub total_owners: usize,
    pub total_revenue: u64,
}
