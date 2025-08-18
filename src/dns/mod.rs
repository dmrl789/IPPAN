//! DNS module for IPPAN
//! 
//! Handles domain name resolution, zone management, and DNS record operations

pub mod apply;
pub mod resolver;
pub mod types;
pub mod validator;
pub mod tld_registry;

use crate::{Result, IppanError};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// DNS system for IPPAN
pub struct DnsSystem {
    /// Zone data storage
    zones: Arc<RwLock<BTreeMap<String, types::Zone>>>,
    /// TLD registry
    tld_registry: Arc<RwLock<tld_registry::TldRegistry>>,
    /// DNS resolver
    resolver: Arc<resolver::ZoneResolver>,
    /// Zone validator
    validator: Arc<validator::ZoneValidator>,
}

impl DnsSystem {
    /// Create a new DNS system
    pub fn new() -> Result<Self> {
        let zones = Arc::new(RwLock::new(BTreeMap::new()));
        let tld_registry = Arc::new(RwLock::new(tld_registry::TldRegistry::new()));
        let resolver = Arc::new(resolver::ZoneResolver::new(zones.clone()));
        let validator = Arc::new(validator::ZoneValidator::new());
        
        Ok(Self {
            zones,
            tld_registry,
            resolver,
            validator,
        })
    }
    
    /// Start the DNS system
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting DNS system...");
        
        // Initialize default zones if needed
        self.initialize_default_zones().await?;
        
        log::info!("DNS system started");
        Ok(())
    }
    
    /// Stop the DNS system
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping DNS system...");
        log::info!("DNS system stopped");
        Ok(())
    }
    
    /// Initialize default zones
    async fn initialize_default_zones(&self) -> Result<()> {
        let mut zones = self.zones.write().await;
        
        // Create default IPPAN zone
        let default_zone = types::Zone {
            domain: "ipn".to_string(),
            owner_pk: [0u8; 32], // System owner
            version: 1,
            serial: 1,
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            rrsets: BTreeMap::new(),
        };
        
        zones.insert("ipn".to_string(), default_zone);
        
        Ok(())
    }
    
    /// Get TLD registry
    pub async fn get_tld_registry(&self) -> tld_registry::TldRegistry {
        self.tld_registry.read().await.clone()
    }
    
    /// Check if TLD is available
    pub async fn is_tld_available(&self, tld: &str) -> bool {
        let registry = self.tld_registry.read().await;
        registry.is_tld_available(tld)
    }
    
    /// Get premium multiplier for TLD
    pub async fn get_tld_multiplier(&self, tld: &str) -> u32 {
        let registry = self.tld_registry.read().await;
        registry.get_premium_multiplier(tld)
    }
    
    /// List available TLDs
    pub async fn list_available_tlds(&self) -> Vec<tld_registry::TldEntry> {
        let registry = self.tld_registry.read().await;
        registry.list_available_tlds()
            .into_iter()
            .cloned()
            .collect()
    }
    
    /// List TLDs by category
    pub async fn list_tlds_by_category(&self, category: &tld_registry::TldCategory) -> Vec<tld_registry::TldEntry> {
        let registry = self.tld_registry.read().await;
        registry.list_tlds_by_category(category)
            .into_iter()
            .cloned()
            .collect()
    }
    
    /// Get DNS resolver
    pub fn get_resolver(&self) -> Arc<resolver::ZoneResolver> {
        self.resolver.clone()
    }
    
    /// Get zone validator
    pub fn get_validator(&self) -> Arc<validator::ZoneValidator> {
        self.validator.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dns_system_creation() {
        let dns_system = DnsSystem::new().unwrap();
        
        // Test TLD registry
        let registry = dns_system.get_tld_registry().await;
        assert!(registry.is_tld_available("ipn"));
        assert!(registry.is_tld_available("ai"));
        assert!(registry.is_tld_available("iot"));
        
        // Test premium multipliers
        assert_eq!(registry.get_premium_multiplier("ipn"), 1);
        assert_eq!(registry.get_premium_multiplier("ai"), 10);
        assert_eq!(registry.get_premium_multiplier("iot"), 2);
    }
    
    #[tokio::test]
    async fn test_dns_system_lifecycle() {
        let dns_system = DnsSystem::new().unwrap();
        
        // Start system
        dns_system.start().await.unwrap();
        
        // Test TLD availability
        assert!(dns_system.is_tld_available("ipn").await);
        assert!(dns_system.is_tld_available("ai").await);
        assert!(!dns_system.is_tld_available("nonexistent").await);
        
        // Test TLD multipliers
        assert_eq!(dns_system.get_tld_multiplier("ipn").await, 1);
        assert_eq!(dns_system.get_tld_multiplier("ai").await, 10);
        assert_eq!(dns_system.get_tld_multiplier("iot").await, 2);
        
        // Stop system
        dns_system.stop().await.unwrap();
    }
}
