//! Premium domain module
//! 
//! Handles premium domain types and features

use crate::Result;
use super::DomainConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Premium domain types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PremiumDomainType {
    /// Human domains
    Human,
    /// Cyborg domains
    Cyborg,
    /// Humanoid domains
    Humanoid,
    /// AI domains
    AI,
    /// IoT domains
    IoT,
    /// Custom premium domain
    Custom(String),
}

/// Premium domain features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumFeatures {
    /// Priority resolution
    pub priority_resolution: bool,
    /// Enhanced security
    pub enhanced_security: bool,
    /// Custom subdomains
    pub custom_subdomains: bool,
    /// Analytics
    pub analytics: bool,
    /// API access
    pub api_access: bool,
    /// Custom features
    pub custom_features: HashMap<String, String>,
}

/// Premium domain manager
pub struct PremiumDomainManager {
    /// Premium domain types
    domain_types: HashMap<String, PremiumDomainType>,
    /// Premium features by type
    features: HashMap<PremiumDomainType, PremiumFeatures>,
    /// Premium pricing
    pricing: HashMap<PremiumDomainType, u64>,
}

/// Premium domains manager
pub struct PremiumDomains {
    /// Domain configuration
    config: DomainConfig,
    /// Premium domain registrations
    premium_domains: RwLock<HashMap<String, PremiumDomainInfo>>,
    /// Premium TLD statistics
    tld_stats: RwLock<HashMap<String, TldStats>>,
}

/// Premium domain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumDomainInfo {
    /// Domain name
    pub domain: String,
    /// TLD
    pub tld: String,
    /// Owner public key
    pub owner: [u8; 32],
    /// Registration timestamp
    pub registered_at: u64,
    /// Premium features
    pub features: Vec<PremiumFeature>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// Premium features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PremiumFeature {
    /// Custom DNS records
    CustomDNS,
    /// Subdomain support
    Subdomains,
    /// Priority resolution
    PriorityResolution,
    /// Enhanced security
    EnhancedSecurity,
    /// Custom branding
    CustomBranding,
}

/// TLD statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TldStats {
    /// TLD name
    pub tld: String,
    /// Number of registered domains
    pub registered_count: u64,
    /// Total revenue from this TLD
    pub total_revenue: u64,
    /// Average domain price
    pub average_price: u64,
    /// Most recent registration
    pub last_registration: Option<u64>,
}

impl PremiumDomains {
    /// Create a new premium domains manager
    pub fn new(config: &DomainConfig) -> Self {
        Self {
            config: config.clone(),
            premium_domains: RwLock::new(HashMap::new()),
            tld_stats: RwLock::new(HashMap::new()),
        }
    }

    /// Register a premium domain
    pub async fn register_premium_domain(
        &self,
        domain: &str,
        owner: [u8; 32],
        features: Vec<PremiumFeature>,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        // Validate that it's a premium TLD
        if !self.is_premium_tld(domain) {
            return Err(crate::IppanError::Domain("Not a premium TLD".to_string()));
        }

        let tld = self.extract_tld(domain)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let premium_info = PremiumDomainInfo {
            domain: domain.to_string(),
            tld: tld.clone(),
            owner,
            registered_at: timestamp,
            features,
            metadata,
        };

        // Store premium domain
        {
            let mut premium_domains = self.premium_domains.write().await;
            premium_domains.insert(domain.to_string(), premium_info);
        }

        // Update TLD statistics
        self.update_tld_stats(&tld, timestamp).await?;

        Ok(())
    }

    /// Get premium domain information
    pub async fn get_premium_domain_info(&self, domain: &str) -> Result<PremiumDomainInfo> {
        let premium_domains = self.premium_domains.read().await;
        premium_domains.get(domain)
            .cloned()
            .ok_or_else(|| crate::IppanError::Domain("Premium domain not found".to_string()))
    }

    /// Check if domain is a premium domain
    pub async fn is_premium_domain(&self, domain: &str) -> Result<bool> {
        let premium_domains = self.premium_domains.read().await;
        Ok(premium_domains.contains_key(domain))
    }

    /// Get premium domains by TLD
    pub async fn get_premium_domains_by_tld(&self, tld: &str) -> Result<Vec<PremiumDomainInfo>> {
        let premium_domains = self.premium_domains.read().await;
        let mut results = Vec::new();

        for domain_info in premium_domains.values() {
            if domain_info.tld == tld {
                results.push(domain_info.clone());
            }
        }

        // Sort by registration date (newest first)
        results.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));

        Ok(results)
    }

    /// Get premium domains by owner
    pub async fn get_premium_domains_by_owner(&self, owner: [u8; 32]) -> Result<Vec<PremiumDomainInfo>> {
        let premium_domains = self.premium_domains.read().await;
        let mut results = Vec::new();

        for domain_info in premium_domains.values() {
            if domain_info.owner == owner {
                results.push(domain_info.clone());
            }
        }

        // Sort by registration date (newest first)
        results.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));

        Ok(results)
    }

    /// Get premium domain count
    pub async fn get_premium_domain_count(&self) -> Result<u64> {
        let premium_domains = self.premium_domains.read().await;
        Ok(premium_domains.len() as u64)
    }

    /// Get TLD statistics
    pub async fn get_tld_stats(&self, tld: &str) -> Result<TldStats> {
        let tld_stats = self.tld_stats.read().await;
        tld_stats.get(tld)
            .cloned()
            .ok_or_else(|| crate::IppanError::Domain("TLD not found".to_string()))
    }

    /// Get all TLD statistics
    pub async fn get_all_tld_stats(&self) -> Result<Vec<TldStats>> {
        let tld_stats = self.tld_stats.read().await;
        Ok(tld_stats.values().cloned().collect())
    }

    /// Update premium domain features
    pub async fn update_premium_features(
        &self,
        domain: &str,
        owner: [u8; 32],
        features: Vec<PremiumFeature>,
    ) -> Result<()> {
        let mut premium_domains = self.premium_domains.write().await;
        let domain_info = premium_domains.get_mut(domain)
            .ok_or_else(|| crate::IppanError::Domain("Premium domain not found".to_string()))?;

        if domain_info.owner != owner {
            return Err(crate::IppanError::Domain("Not the domain owner".to_string()));
        }

        domain_info.features = features;
        Ok(())
    }

    /// Update premium domain metadata
    pub async fn update_premium_metadata(
        &self,
        domain: &str,
        owner: [u8; 32],
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        let mut premium_domains = self.premium_domains.write().await;
        let domain_info = premium_domains.get_mut(domain)
            .ok_or_else(|| crate::IppanError::Domain("Premium domain not found".to_string()))?;

        if domain_info.owner != owner {
            return Err(crate::IppanError::Domain("Not the domain owner".to_string()));
        }

        domain_info.metadata = metadata;
        Ok(())
    }

    /// Get premium domain price
    pub async fn get_premium_domain_price(&self, domain: &str) -> Result<u64> {
        if !self.is_premium_tld(domain) {
            return Err(crate::IppanError::Domain("Not a premium TLD".to_string()));
        }

        let base_price = self.config.registration_fee;
        let premium_price = (base_price as f64 * self.config.premium_multiplier) as u64;
        Ok(premium_price)
    }

    /// Get premium features for a domain
    pub async fn get_premium_features(&self, domain: &str) -> Result<Vec<PremiumFeature>> {
        let premium_domains = self.premium_domains.read().await;
        let domain_info = premium_domains.get(domain)
            .ok_or_else(|| crate::IppanError::Domain("Premium domain not found".to_string()))?;

        Ok(domain_info.features.clone())
    }

    /// Check if domain has a specific premium feature
    pub async fn has_premium_feature(&self, domain: &str, feature: PremiumFeature) -> Result<bool> {
        let features = self.get_premium_features(domain).await?;
        Ok(features.contains(&feature))
    }

    /// Get premium domain statistics
    pub async fn get_premium_domain_stats(&self) -> Result<PremiumDomainStats> {
        let premium_domains = self.premium_domains.read().await;
        let tld_stats = self.tld_stats.read().await;

        let total_premium_domains = premium_domains.len() as u64;
        let total_tlds = tld_stats.len() as u64;
        let mut total_revenue = 0;

        for stats in tld_stats.values() {
            total_revenue += stats.total_revenue;
        }

        let mut tld_distribution = HashMap::new();
        for domain_info in premium_domains.values() {
            *tld_distribution.entry(domain_info.tld.clone()).or_insert(0) += 1;
        }

        Ok(PremiumDomainStats {
            total_premium_domains,
            total_tlds,
            total_revenue,
            tld_distribution,
        })
    }

    /// Check if domain has a premium TLD
    fn is_premium_tld(&self, domain: &str) -> bool {
        if let Some(tld) = self.extract_tld(domain).ok() {
            self.config.premium_tlds.contains(&tld)
        } else {
            false
        }
    }

    /// Extract TLD from domain name
    fn extract_tld(&self, domain: &str) -> Result<String> {
        domain.split('.')
            .nth(1)
            .map(|tld| tld.to_string())
            .ok_or_else(|| crate::IppanError::Domain("Invalid domain format".to_string()))
    }

    /// Update TLD statistics
    async fn update_tld_stats(&self, tld: &str, timestamp: u64) -> Result<()> {
        let mut tld_stats = self.tld_stats.write().await;
        let stats = tld_stats.entry(tld.to_string()).or_insert_with(|| TldStats {
            tld: tld.to_string(),
            registered_count: 0,
            total_revenue: 0,
            average_price: 0,
            last_registration: None,
        });

        stats.registered_count += 1;
        stats.last_registration = Some(timestamp);

        // Calculate average price (simplified)
        let base_price = self.config.registration_fee;
        let premium_price = (base_price as f64 * self.config.premium_multiplier) as u64;
        stats.total_revenue += premium_price;
        stats.average_price = stats.total_revenue / stats.registered_count;

        Ok(())
    }
}

/// Premium domain statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumDomainStats {
    /// Total number of premium domains
    pub total_premium_domains: u64,
    /// Number of premium TLDs
    pub total_tlds: u64,
    /// Total revenue from premium domains
    pub total_revenue: u64,
    /// Distribution by TLD
    pub tld_distribution: HashMap<String, u64>,
}

impl PremiumDomainManager {
    /// Create a new premium domain manager
    pub fn new() -> Self {
        let mut domain_types = HashMap::new();
        domain_types.insert(".m".to_string(), PremiumDomainType::Human);
        domain_types.insert(".cyborg".to_string(), PremiumDomainType::Cyborg);
        domain_types.insert(".humanoid".to_string(), PremiumDomainType::Humanoid);
        domain_types.insert(".ai".to_string(), PremiumDomainType::AI);
        domain_types.insert(".iot".to_string(), PremiumDomainType::IoT);
        
        let mut features = HashMap::new();
        features.insert(PremiumDomainType::Human, PremiumFeatures {
            priority_resolution: true,
            enhanced_security: true,
            custom_subdomains: false,
            analytics: false,
            api_access: false,
            custom_features: HashMap::new(),
        });
        
        features.insert(PremiumDomainType::Cyborg, PremiumFeatures {
            priority_resolution: true,
            enhanced_security: true,
            custom_subdomains: true,
            analytics: true,
            api_access: false,
            custom_features: HashMap::new(),
        });
        
        features.insert(PremiumDomainType::Humanoid, PremiumFeatures {
            priority_resolution: true,
            enhanced_security: true,
            custom_subdomains: true,
            analytics: true,
            api_access: true,
            custom_features: HashMap::new(),
        });
        
        features.insert(PremiumDomainType::AI, PremiumFeatures {
            priority_resolution: true,
            enhanced_security: true,
            custom_subdomains: true,
            analytics: true,
            api_access: true,
            custom_features: HashMap::new(),
        });
        
        features.insert(PremiumDomainType::IoT, PremiumFeatures {
            priority_resolution: false,
            enhanced_security: true,
            custom_subdomains: false,
            analytics: true,
            api_access: true,
            custom_features: HashMap::new(),
        });
        
        let mut pricing = HashMap::new();
        pricing.insert(PremiumDomainType::Human, 10_000_000); // 0.1 IPN
        pricing.insert(PremiumDomainType::Cyborg, 25_000_000); // 0.25 IPN
        pricing.insert(PremiumDomainType::Humanoid, 50_000_000); // 0.5 IPN
        pricing.insert(PremiumDomainType::AI, 100_000_000); // 1.0 IPN
        pricing.insert(PremiumDomainType::IoT, 5_000_000); // 0.05 IPN
        
        Self {
            domain_types,
            features,
            pricing,
        }
    }
    
    /// Check if domain is premium
    pub fn is_premium_domain(&self, domain: &str) -> bool {
        for (tld, _) in &self.domain_types {
            if domain.ends_with(tld) {
                return true;
            }
        }
        false
    }
    
    /// Get premium domain type
    pub fn get_premium_type(&self, domain: &str) -> Option<PremiumDomainType> {
        for (tld, domain_type) in &self.domain_types {
            if domain.ends_with(tld) {
                return Some(domain_type.clone());
            }
        }
        None
    }
    
    /// Get premium features
    pub fn get_features(&self, domain_type: &PremiumDomainType) -> Option<PremiumFeatures> {
        self.features.get(domain_type).cloned()
    }
    
    /// Get premium pricing
    pub fn get_pricing(&self, domain_type: &PremiumDomainType) -> Option<u64> {
        self.pricing.get(domain_type).cloned()
    }
    
    /// Add custom premium domain type
    pub fn add_custom_type(&mut self, tld: String, features: PremiumFeatures, price: u64) -> Result<()> {
        let domain_type = PremiumDomainType::Custom(tld.clone());
        self.domain_types.insert(tld, domain_type.clone());
        self.features.insert(domain_type.clone(), features);
        self.pricing.insert(domain_type, price);
        Ok(())
    }
    
    /// Get all premium domain types
    pub fn get_all_types(&self) -> Vec<PremiumDomainType> {
        self.domain_types.values().cloned().collect()
    }
    
    /// Get all premium TLDs
    pub fn get_all_tlds(&self) -> Vec<String> {
        self.domain_types.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_premium_domain_registration() {
        let config = DomainConfig::default();
        let premium = PremiumDomains::new(&config);

        let domain = "alice.m";
        let owner = [1u8; 32];
        let features = vec![PremiumFeature::CustomDNS, PremiumFeature::Subdomains];
        let mut metadata = HashMap::new();
        metadata.insert("description".to_string(), "Premium domain".to_string());

        // Register premium domain
        premium.register_premium_domain(domain, owner, features.clone(), metadata.clone()).await.unwrap();

        // Check if it's a premium domain
        assert!(premium.is_premium_domain(domain).await.unwrap());

        // Get premium domain info
        let domain_info = premium.get_premium_domain_info(domain).await.unwrap();
        assert_eq!(domain_info.owner, owner);
        assert_eq!(domain_info.features, features);
    }

    #[tokio::test]
    async fn test_premium_tld_detection() {
        let config = DomainConfig::default();
        let premium = PremiumDomains::new(&config);

        // Test premium TLDs
        assert!(premium.is_premium_tld("alice.m"));
        assert!(premium.is_premium_tld("bob.cyborg"));
        assert!(premium.is_premium_tld("charlie.humanoid"));

        // Test regular TLD
        assert!(!premium.is_premium_tld("alice.ipn"));
    }

    #[tokio::test]
    async fn test_premium_domain_price() {
        let config = DomainConfig::default();
        let premium = PremiumDomains::new(&config);

        let price = premium.get_premium_domain_price("alice.m").await.unwrap();
        let expected_price = (config.registration_fee as f64 * config.premium_multiplier) as u64;
        assert_eq!(price, expected_price);
    }
}
