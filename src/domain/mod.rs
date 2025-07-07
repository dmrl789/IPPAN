//! Domain module for IPPAN
//! 
//! Handles domain name registration, renewal, and management

pub mod fees;
pub mod premium;
pub mod registry;
pub mod renewals;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Domain manager
pub struct DomainManager {
    /// Domain registry
    registry: Arc<RwLock<HashMap<String, DomainRecord>>>,
    /// Domain fees
    fees: Arc<RwLock<HashMap<String, u64>>>,
    /// Premium domains
    premium_domains: Arc<RwLock<HashMap<String, PremiumDomain>>>,
}

/// Domain record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRecord {
    /// Domain name
    pub name: String,
    /// Owner address
    pub owner: [u8; 32],
    /// Registration timestamp
    pub registered_at: u64,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Domain status
    pub status: DomainStatus,
    /// Domain type
    pub domain_type: DomainType,
    /// Associated data
    pub data: Option<Vec<u8>>,
}

/// Domain status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainStatus {
    /// Domain is active
    Active,
    /// Domain is expired
    Expired,
    /// Domain is suspended
    Suspended,
    /// Domain is pending
    Pending,
}

/// Domain type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainType {
    /// Standard domain (.ipn)
    Standard,
    /// Premium domain (.m, .cyborg, .humanoid)
    Premium,
    /// IoT domain (.iot)
    IoT,
    /// AI domain (.ai)
    AI,
}

/// Premium domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumDomain {
    /// Domain name
    pub name: String,
    /// Premium type
    pub premium_type: String,
    /// Annual fee
    pub annual_fee: u64,
    /// Features
    pub features: Vec<String>,
}

impl DomainManager {
    /// Create a new domain manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            fees: Arc::new(RwLock::new(HashMap::new())),
            premium_domains: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a domain
    pub async fn register_domain(
        &self,
        name: String,
        owner: [u8; 32],
        domain_type: DomainType,
        duration_years: u64,
    ) -> Result<()> {
        // Validate domain name
        self.validate_domain_name(&name)?;
        
        // Check if domain is available
        if self.is_domain_registered(&name).await {
            return Err(crate::error::IppanError::Domain(
                format!("Domain {} is already registered", name)
            ));
        }
        
        // Calculate registration fee
        let fee = self.calculate_registration_fee(&name, &domain_type, duration_years).await?;
        
        // Create domain record
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expires_at = now + (duration_years * 365 * 24 * 60 * 60);
        
        let record = DomainRecord {
            name: name.clone(),
            owner,
            registered_at: now,
            expires_at,
            status: DomainStatus::Active,
            domain_type,
            data: None,
        };
        
        // Add to registry
        let mut registry = self.registry.write().await;
        registry.insert(name, record);
        
        Ok(())
    }
    
    /// Renew a domain
    pub async fn renew_domain(&self, name: &str, owner: [u8; 32], years: u64) -> Result<()> {
        let mut registry = self.registry.write().await;
        
        if let Some(record) = registry.get_mut(name) {
            // Verify ownership
            if record.owner != owner {
                return Err(crate::error::IppanError::Domain(
                    "Domain ownership verification failed".to_string()
                ));
            }
            
            // Calculate renewal fee
            let fee = self.calculate_renewal_fee(name, &record.domain_type, years).await?;
            
            // Extend expiration
            record.expires_at += years * 365 * 24 * 60 * 60;
            record.status = DomainStatus::Active;
            
            Ok(())
        } else {
            Err(crate::error::IppanError::Domain(
                format!("Domain {} not found", name)
            ))
        }
    }
    
    /// Transfer domain ownership
    pub async fn transfer_domain(&self, name: &str, current_owner: [u8; 32], new_owner: [u8; 32]) -> Result<()> {
        let mut registry = self.registry.write().await;
        
        if let Some(record) = registry.get_mut(name) {
            // Verify current ownership
            if record.owner != current_owner {
                return Err(crate::error::IppanError::Domain(
                    "Domain ownership verification failed".to_string()
                ));
            }
            
            // Transfer ownership
            record.owner = new_owner;
            
            Ok(())
        } else {
            Err(crate::error::IppanError::Domain(
                format!("Domain {} not found", name)
            ))
        }
    }
    
    /// Get domain record
    pub async fn get_domain(&self, name: &str) -> Option<DomainRecord> {
        let registry = self.registry.read().await;
        registry.get(name).cloned()
    }
    
    /// Check if domain is registered
    pub async fn is_domain_registered(&self, name: &str) -> bool {
        let registry = self.registry.read().await;
        registry.contains_key(name)
    }
    
    /// Get domains by owner
    pub async fn get_domains_by_owner(&self, owner: [u8; 32]) -> Vec<DomainRecord> {
        let registry = self.registry.read().await;
        registry
            .values()
            .filter(|record| record.owner == owner)
            .cloned()
            .collect()
    }
    
    /// Get expired domains
    pub async fn get_expired_domains(&self) -> Vec<DomainRecord> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let registry = self.registry.read().await;
        registry
            .values()
            .filter(|record| record.expires_at < now)
            .cloned()
            .collect()
    }
    
    /// Validate domain name
    fn validate_domain_name(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(crate::error::IppanError::Domain(
                "Domain name cannot be empty".to_string()
            ));
        }
        
        if name.len() > 63 {
            return Err(crate::error::IppanError::Domain(
                "Domain name too long".to_string()
            ));
        }
        
        // Check for valid characters
        for c in name.chars() {
            if !c.is_alphanumeric() && c != '-' && c != '.' {
                return Err(crate::error::IppanError::Domain(
                    format!("Invalid character in domain name: {}", c)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Calculate registration fee
    async fn calculate_registration_fee(&self, name: &str, domain_type: &DomainType, years: u64) -> Result<u64> {
        let base_fee = match domain_type {
            DomainType::Standard => 1_000_000, // 0.01 IPN
            DomainType::Premium => 10_000_000, // 0.1 IPN
            DomainType::IoT => 5_000_000, // 0.05 IPN
            DomainType::AI => 15_000_000, // 0.15 IPN
        };
        
        Ok(base_fee * years)
    }
    
    /// Calculate renewal fee
    async fn calculate_renewal_fee(&self, name: &str, domain_type: &DomainType, years: u64) -> Result<u64> {
        self.calculate_registration_fee(name, domain_type, years).await
    }
    
    /// Add premium domain
    pub async fn add_premium_domain(&self, name: String, premium_type: String, annual_fee: u64) -> Result<()> {
        let premium_domain = PremiumDomain {
            name: name.clone(),
            premium_type,
            annual_fee,
            features: Vec::new(),
        };
        
        let mut premium_domains = self.premium_domains.write().await;
        premium_domains.insert(name, premium_domain);
        
        Ok(())
    }
    
    /// Get premium domain
    pub async fn get_premium_domain(&self, name: &str) -> Option<PremiumDomain> {
        let premium_domains = self.premium_domains.read().await;
        premium_domains.get(name).cloned()
    }
}
