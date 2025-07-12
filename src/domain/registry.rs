//! Domain registry module
//! 
//! Handles domain name registration and management

use crate::{
    wallet::WalletManager,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Domain registry for managing domain registrations
pub struct DomainRegistry {
    /// Registered domains
    domains: HashMap<String, DomainRecord>,
    /// Domain ownership by address
    ownership: HashMap<String, Vec<String>>,
    /// Wallet for fee collection
    wallet: Arc<RwLock<WalletManager>>,
    /// Total revenue collected
    total_revenue: u64,
}

impl DomainRegistry {
    pub fn new(wallet: Arc<RwLock<WalletManager>>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            domains: HashMap::new(),
            ownership: HashMap::new(),
            wallet,
            total_revenue: 0,
        })
    }

    /// Start the domain registry
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting domain registry...");
        
        // Load existing domains from storage
        self.load_domains().await?;
        
        log::info!("Domain registry started");
        Ok(())
    }

    /// Stop the domain registry
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping domain registry...");
        
        // Save domains to storage
        self.save_domains().await?;
        
        log::info!("Domain registry stopped");
        Ok(())
    }

    /// Register a new domain
    pub async fn register_domain(&mut self, handle: &str, owner_address: &str, duration_years: u32) -> Result<crate::domain::DomainRegistration, crate::domain::DomainError> {
        // Check if domain is already registered
        if self.domains.contains_key(handle) {
            return Err(crate::domain::DomainError::DomainAlreadyRegistered { handle: handle.to_string() });
        }
        
        // Create domain record
        let registration_date = SystemTime::now();
        let expiry_date = registration_date + Duration::from_secs(duration_years as u64 * 365 * 24 * 60 * 60);
        
        let domain_record = DomainRecord {
            handle: handle.to_string(),
            owner_address: owner_address.to_string(),
            registration_date,
            expiry_date,
            is_active: true,
            transfer_count: 0,
            renewal_count: 0,
            registration_fee: self.calculate_registration_fee(duration_years),
        };
        
        // Add to domains
        self.domains.insert(handle.to_string(), domain_record.clone());
        
        // Update ownership index
        self.ownership.entry(owner_address.to_string())
            .or_insert_with(Vec::new)
            .push(handle.to_string());
        
        // Update revenue
        self.total_revenue += domain_record.registration_fee;
        
        // Add fee to Global Fund (wallet)
        let mut wallet = self.wallet.write().await;
        wallet.add_domain_fee(domain_record.registration_fee).await?;
        
        Ok(crate::domain::DomainRegistration {
            handle: handle.to_string(),
            owner_address: owner_address.to_string(),
            registration_date,
            expiry_date,
            duration_years,
            fee_paid: domain_record.registration_fee,
        })
    }

    /// Transfer domain ownership
    pub async fn transfer_domain(&mut self, handle: &str, new_owner: &str) -> Result<crate::domain::DomainTransfer, crate::domain::DomainError> {
        let domain_record = self.domains.get_mut(handle)
            .ok_or_else(|| crate::domain::DomainError::DomainNotFound { handle: handle.to_string() })?;
        
        let previous_owner = domain_record.owner_address.clone();
        
        // Remove from previous owner's list
        if let Some(domains) = self.ownership.get_mut(&previous_owner) {
            domains.retain(|d| d != handle);
        }
        
        // Update domain record
        domain_record.owner_address = new_owner.to_string();
        domain_record.transfer_count += 1;
        
        // Add to new owner's list
        self.ownership.entry(new_owner.to_string())
            .or_insert_with(Vec::new)
            .push(handle.to_string());
        
        Ok(crate::domain::DomainTransfer {
            handle: handle.to_string(),
            previous_owner,
            new_owner: new_owner.to_string(),
            transfer_date: SystemTime::now(),
        })
    }

    /// Get domain information
    pub fn get_domain_info(&self, handle: &str) -> Option<crate::domain::DomainInfo> {
        self.domains.get(handle).map(|record| crate::domain::DomainInfo {
            handle: record.handle.clone(),
            owner_address: record.owner_address.clone(),
            registration_date: record.registration_date,
            expiry_date: record.expiry_date,
            is_active: record.is_active,
            transfer_count: record.transfer_count,
            renewal_count: record.renewal_count,
        })
    }

    /// Check if domain is available
    pub fn is_domain_available(&self, handle: &str) -> bool {
        !self.domains.contains_key(handle)
    }

    /// Get domains owned by an address
    pub fn get_domains_by_owner(&self, owner_address: &str) -> Vec<crate::domain::DomainInfo> {
        self.ownership.get(owner_address)
            .map(|handles| {
                handles.iter()
                    .filter_map(|handle| self.get_domain_info(handle))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total number of domains
    pub fn get_total_domains(&self) -> u64 {
        self.domains.len() as u64
    }

    /// Get total revenue
    pub fn get_total_revenue(&self) -> u64 {
        self.total_revenue
    }

    /// Update domain expiry date (called by renewal manager)
    pub fn update_domain_expiry(&mut self, handle: &str, new_expiry: SystemTime) -> Result<(), crate::domain::DomainError> {
        let domain_record = self.domains.get_mut(handle)
            .ok_or_else(|| crate::domain::DomainError::DomainNotFound { handle: handle.to_string() })?;
        
        domain_record.expiry_date = new_expiry;
        domain_record.renewal_count += 1;
        
        Ok(())
    }

    /// Mark domain as expired
    pub fn mark_domain_expired(&mut self, handle: &str) -> Result<(), crate::domain::DomainError> {
        let domain_record = self.domains.get_mut(handle)
            .ok_or_else(|| crate::domain::DomainError::DomainNotFound { handle: handle.to_string() })?;
        
        domain_record.is_active = false;
        
        Ok(())
    }

    /// Get expired domains
    pub fn get_expired_domains(&self) -> Vec<crate::domain::DomainInfo> {
        let now = SystemTime::now();
        self.domains.values()
            .filter(|record| record.expiry_date < now)
            .map(|record| crate::domain::DomainInfo {
                handle: record.handle.clone(),
                owner_address: record.owner_address.clone(),
                registration_date: record.registration_date,
                expiry_date: record.expiry_date,
                is_active: record.is_active,
                transfer_count: record.transfer_count,
                renewal_count: record.renewal_count,
            })
            .collect()
    }

    /// Get domains expiring within days
    pub fn get_domains_expiring_within(&self, days: u32) -> Vec<crate::domain::DomainInfo> {
        let now = SystemTime::now();
        let threshold = now + Duration::from_secs(days as u64 * 24 * 60 * 60);
        
        self.domains.values()
            .filter(|record| record.expiry_date > now && record.expiry_date <= threshold)
            .map(|record| crate::domain::DomainInfo {
                handle: record.handle.clone(),
                owner_address: record.owner_address.clone(),
                registration_date: record.registration_date,
                expiry_date: record.expiry_date,
                is_active: record.is_active,
                transfer_count: record.transfer_count,
                renewal_count: record.renewal_count,
            })
            .collect()
    }

    /// Calculate registration fee
    fn calculate_registration_fee(&self, duration_years: u32) -> u64 {
        let base_fee = 1_000_000; // 1 IPN in smallest units
        base_fee * duration_years as u64
    }

    /// Load domains from storage
    async fn load_domains(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would load from persistent storage
        // For now, start with empty registry
        log::info!("Loading domains from storage...");
        Ok(())
    }

    /// Save domains to storage
    async fn save_domains(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation would save to persistent storage
        log::info!("Saving {} domains to storage...", self.domains.len());
        Ok(())
    }
}

/// Domain record stored in registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRecord {
    pub handle: String,
    pub owner_address: String,
    pub registration_date: SystemTime,
    pub expiry_date: SystemTime,
    pub is_active: bool,
    pub transfer_count: u32,
    pub renewal_count: u32,
    pub registration_fee: u64,
}
