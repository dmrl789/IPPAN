//! Domain module for IPPAN
//! 
//! Handles domain name registration, renewal, and management

pub mod registry;
pub mod renewals;
pub mod premium;

use crate::{
    wallet::WalletManager,
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Domain system for human-readable handles
pub struct DomainSystem {
    /// Domain registry management
    pub registry: Arc<RwLock<registry::DomainRegistry>>,
    /// Renewal management
    pub renewals: Arc<RwLock<renewals::RenewalManager>>,
    /// Premium TLD management
    pub premium: Arc<RwLock<premium::PremiumTldManager>>,
    /// Wallet for fee collection
    wallet: Arc<RwLock<WalletManager>>,
    /// Domain fees configuration
    fees: DomainFees,
}

impl DomainSystem {
    pub fn new(
        wallet: Arc<RwLock<WalletManager>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Arc::new(RwLock::new(registry::DomainRegistry::new(wallet.clone())?));
        let renewals = Arc::new(RwLock::new(renewals::RenewalManager::new(wallet.clone())?));
        let premium = Arc::new(RwLock::new(premium::PremiumTldManager::new(wallet.clone())?));
        
        let fees = DomainFees {
            registration_fee: 1_000_000, // 1 IPN in smallest units
            renewal_fee: 500_000, // 0.5 IPN in smallest units
            premium_tld_fee: 10_000_000, // 10 IPN for premium TLDs
        };
        
        Ok(Self {
            registry,
            renewals,
            premium,
            wallet,
            fees,
        })
    }

    /// Start the domain system
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting domain system...");
        
        self.registry.write().await.start().await?;
        self.renewals.write().await.start().await?;
        self.premium.write().await.start().await?;
        
        log::info!("Domain system started");
        Ok(())
    }

    /// Stop the domain system
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Stopping domain system...");
        
        self.premium.write().await.stop().await?;
        self.renewals.write().await.stop().await?;
        self.registry.write().await.stop().await?;
        
        log::info!("Domain system stopped");
        Ok(())
    }

    /// Register a new domain
    pub async fn register_domain(&self, handle: &str, owner_address: &str, duration_years: u32) -> Result<DomainRegistration, DomainError> {
        // Validate handle format
        self.validate_handle(handle)?;
        
        // Check if domain is available
        if !self.is_domain_available(handle).await? {
            return Err(DomainError::DomainAlreadyRegistered { handle: handle.to_string() });
        }
        
        // Calculate registration fee
        let total_fee = self.fees.registration_fee * duration_years as u64;
        
        // Check wallet balance
        let wallet = self.wallet.read().await;
        if wallet.get_balance() < total_fee {
            return Err(DomainError::InsufficientBalance {
                required: total_fee,
                available: wallet.get_balance(),
            });
        }
        drop(wallet);
        
        // Register domain
        let mut registry = self.registry.write().await;
        let registration = registry.register_domain(handle, owner_address, duration_years).await?;
        
        // Deduct fee from wallet
        let mut wallet = self.wallet.write().await;
        wallet.deduct_balance(total_fee).await?;
        
        log::info!("Registered domain {} for {} years", handle, duration_years);
        Ok(registration)
    }

    /// Renew a domain
    pub async fn renew_domain(&self, handle: &str, duration_years: u32) -> Result<DomainRenewal, DomainError> {
        // Check if domain exists
        let domain_info = self.get_domain_info(handle).await?
            .ok_or_else(|| DomainError::DomainNotFound { handle: handle.to_string() })?;
        
        // Check if caller is the owner
        let wallet = self.wallet.read().await;
        let caller_address = wallet.get_primary_address();
        if domain_info.owner_address != caller_address {
            return Err(DomainError::NotDomainOwner { handle: handle.to_string() });
        }
        drop(wallet);
        
        // Calculate renewal fee
        let total_fee = self.fees.renewal_fee * duration_years as u64;
        
        // Check wallet balance
        let wallet = self.wallet.read().await;
        if wallet.get_balance() < total_fee {
            return Err(DomainError::InsufficientBalance {
                required: total_fee,
                available: wallet.get_balance(),
            });
        }
        drop(wallet);
        
        // Renew domain
        let mut renewals = self.renewals.write().await;
        let renewal = renewals.renew_domain(handle, duration_years).await?;
        
        // Deduct fee from wallet
        let mut wallet = self.wallet.write().await;
        wallet.deduct_balance(total_fee).await?;
        
        log::info!("Renewed domain {} for {} years", handle, duration_years);
        Ok(renewal)
    }

    /// Transfer domain ownership
    pub async fn transfer_domain(&self, handle: &str, new_owner: &str) -> Result<DomainTransfer, DomainError> {
        // Check if domain exists
        let domain_info = self.get_domain_info(handle).await?
            .ok_or_else(|| DomainError::DomainNotFound { handle: handle.to_string() })?;
        
        // Check if caller is the owner
        let wallet = self.wallet.read().await;
        let caller_address = wallet.get_primary_address();
        if domain_info.owner_address != caller_address {
            return Err(DomainError::NotDomainOwner { handle: handle.to_string() });
        }
        drop(wallet);
        
        // Transfer domain
        let mut registry = self.registry.write().await;
        let transfer = registry.transfer_domain(handle, new_owner).await?;
        
        log::info!("Transferred domain {} to {}", handle, new_owner);
        Ok(transfer)
    }

    /// Get domain information
    pub async fn get_domain_info(&self, handle: &str) -> Result<Option<DomainInfo>, DomainError> {
        let registry = self.registry.read().await;
        Ok(registry.get_domain_info(handle))
    }

    /// Check if domain is available
    pub async fn is_domain_available(&self, handle: &str) -> Result<bool, DomainError> {
        let registry = self.registry.read().await;
        Ok(registry.is_domain_available(handle))
    }

    /// Get domains owned by an address
    pub async fn get_domains_by_owner(&self, owner_address: &str) -> Result<Vec<DomainInfo>, DomainError> {
        let registry = self.registry.read().await;
        Ok(registry.get_domains_by_owner(owner_address))
    }

    /// Get expiring domains
    pub async fn get_expiring_domains(&self, days: u32) -> Result<Vec<DomainInfo>, DomainError> {
        let renewals = self.renewals.read().await;
        Ok(renewals.get_expiring_domains(days))
    }

    /// Register a premium TLD
    pub async fn register_premium_tld(&self, tld: &str, owner_address: &str) -> Result<PremiumTldRegistration, DomainError> {
        // Validate TLD format
        self.validate_tld(tld)?;
        
        // Check if TLD is available
        if !self.is_premium_tld_available(tld).await? {
            return Err(DomainError::TldAlreadyRegistered { tld: tld.to_string() });
        }
        
        // Check wallet balance for premium fee
        let wallet = self.wallet.read().await;
        if wallet.get_balance() < self.fees.premium_tld_fee {
            return Err(DomainError::InsufficientBalance {
                required: self.fees.premium_tld_fee,
                available: wallet.get_balance(),
            });
        }
        drop(wallet);
        
        // Register premium TLD
        let mut premium = self.premium.write().await;
        let registration = premium.register_tld(tld, owner_address).await?;
        
        // Deduct fee from wallet
        let mut wallet = self.wallet.write().await;
        wallet.deduct_balance(self.fees.premium_tld_fee).await?;
        
        log::info!("Registered premium TLD {} for {}", tld, owner_address);
        Ok(registration)
    }

    /// Get domain statistics
    pub async fn get_domain_stats(&self) -> DomainStats {
        let registry = self.registry.read().await;
        let renewals = self.renewals.read().await;
        let premium = self.premium.read().await;
        
        DomainStats {
            total_domains: registry.get_total_domains(),
            total_premium_tlds: premium.get_total_tlds(),
            expiring_this_month: renewals.get_expiring_domains(30).len(),
            total_revenue: registry.get_total_revenue() + premium.get_total_revenue(),
        }
    }

    /// Validate handle format
    fn validate_handle(&self, handle: &str) -> Result<(), DomainError> {
        if handle.is_empty() || handle.len() > 63 {
            return Err(DomainError::InvalidHandleFormat { handle: handle.to_string() });
        }
        
        // Check for valid characters (alphanumeric, hyphens, dots)
        if !handle.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.') {
            return Err(DomainError::InvalidHandleFormat { handle: handle.to_string() });
        }
        
        // Check for valid TLD
        if !handle.contains('.') {
            return Err(DomainError::InvalidHandleFormat { handle: handle.to_string() });
        }
        
        Ok(())
    }

    /// Validate TLD format
    fn validate_tld(&self, tld: &str) -> Result<(), DomainError> {
        if tld.is_empty() || tld.len() > 10 {
            return Err(DomainError::InvalidTldFormat { tld: tld.to_string() });
        }
        
        // Check for valid characters (alphanumeric, hyphens)
        if !tld.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(DomainError::InvalidTldFormat { tld: tld.to_string() });
        }
        
        Ok(())
    }
}

/// Domain registration result
#[derive(Debug, Serialize)]
pub struct DomainRegistration {
    pub handle: String,
    pub owner_address: String,
    pub registration_date: SystemTime,
    pub expiry_date: SystemTime,
    pub duration_years: u32,
    pub fee_paid: u64,
}

/// Domain renewal result
#[derive(Debug, Serialize)]
pub struct DomainRenewal {
    pub handle: String,
    pub renewal_date: SystemTime,
    pub new_expiry_date: SystemTime,
    pub duration_years: u32,
    pub fee_paid: u64,
}

/// Domain transfer result
#[derive(Debug, Serialize)]
pub struct DomainTransfer {
    pub handle: String,
    pub previous_owner: String,
    pub new_owner: String,
    pub transfer_date: SystemTime,
}

/// Premium TLD registration result
#[derive(Debug, Serialize)]
pub struct PremiumTldRegistration {
    pub tld: String,
    pub owner_address: String,
    pub registration_date: SystemTime,
    pub fee_paid: u64,
}

/// Domain information
#[derive(Debug, Serialize, Clone)]
pub struct DomainInfo {
    pub handle: String,
    pub owner_address: String,
    pub registration_date: SystemTime,
    pub expiry_date: SystemTime,
    pub is_active: bool,
    pub transfer_count: u32,
    pub renewal_count: u32,
}

/// Domain statistics
#[derive(Debug, Serialize)]
pub struct DomainStats {
    pub total_domains: u64,
    pub total_premium_tlds: u64,
    pub expiring_this_month: usize,
    pub total_revenue: u64,
}

/// Domain fees configuration
#[derive(Debug, Clone)]
pub struct DomainFees {
    pub registration_fee: u64,
    pub renewal_fee: u64,
    pub premium_tld_fee: u64,
}

/// Domain errors
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid handle format: {handle}")]
    InvalidHandleFormat { handle: String },
    
    #[error("Invalid TLD format: {tld}")]
    InvalidTldFormat { tld: String },
    
    #[error("Domain already registered: {handle}")]
    DomainAlreadyRegistered { handle: String },
    
    #[error("Domain not found: {handle}")]
    DomainNotFound { handle: String },
    
    #[error("TLD already registered: {tld}")]
    TldAlreadyRegistered { tld: String },
    
    #[error("Not domain owner: {handle}")]
    NotDomainOwner { handle: String },
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("Domain expired: {handle}")]
    DomainExpired { handle: String },
    
    #[error("Registration failed: {reason}")]
    RegistrationFailed { reason: String },
    
    #[error("Renewal failed: {reason}")]
    RenewalFailed { reason: String },
    
    #[error("Transfer failed: {reason}")]
    TransferFailed { reason: String },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
}
