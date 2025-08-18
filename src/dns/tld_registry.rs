//! IPPAN Top-Level Domain Registry
//! 
//! Comprehensive registry of available TLDs for IPPAN with premium multipliers,
//! categories, and descriptions. This registry excludes existing DNS TLDs and
//! ISO country codes to avoid conflicts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// TLD category for classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TldCategory {
    /// Standard domains (base pricing)
    Standard,
    /// Premium/short domains (high multiplier)
    Premium,
    /// Technology-focused domains
    Tech,
    /// IoT and machine-to-machine domains
    IoT,
    /// Financial and payment domains
    Finance,
    /// Decentralized and blockchain domains
    Decentralized,
    /// Storage and data domains
    Storage,
    /// Development and infrastructure domains
    Development,
}

/// TLD registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TldEntry {
    /// TLD name (without dot)
    pub tld: String,
    /// Premium multiplier for fee calculation
    pub premium_multiplier: u32,
    /// Category classification
    pub category: TldCategory,
    /// Human-readable description
    pub description: String,
    /// Whether this TLD is available for registration
    pub available: bool,
    /// Minimum registration years
    pub min_years: u32,
    /// Maximum registration years
    pub max_years: u32,
}

/// IPPAN TLD Registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TldRegistry {
    /// All registered TLDs
    pub tlds: HashMap<String, TldEntry>,
    /// Default TLD for IPPAN
    pub default_tld: String,
}

impl TldRegistry {
    /// Create the default IPPAN TLD registry
    pub fn new() -> Self {
        let mut registry = Self {
            tlds: HashMap::new(),
            default_tld: "ipn".to_string(),
        };
        
        registry.initialize_tlds();
        registry
    }
    
    /// Initialize all available TLDs
    fn initialize_tlds(&mut self) {
        // 1-letter TLDs (Premium - ×10 multiplier)
        let one_letter_tlds = vec![
            "a", "b", "c", "d", "e", "f", "g", "h", "j", "k", "l", "n", 
            "o", "p", "q", "r", "t", "u", "v", "w", "x", "y", "z"
        ];
        
        for tld in one_letter_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 10,
                category: TldCategory::Premium,
                description: format!("Premium 1-letter TLD - {}", tld.to_uppercase()),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // 2-letter TLDs (Premium - ×10 multiplier)
        let two_letter_tlds = vec![
            "aa", "aq", "aw", "bx", "cq", "cy", "dx", "eh", "fb", "fy", "gx", "ii", 
            "iw", "jq", "kx", "lq", "mq", "ns", "oa", "pb", "qc", "qx", "rr", 
            "sx", "ti", "uq", "vb", "wc", "ww", "xy", "yq", "zz"
        ];
        
        for tld in two_letter_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 10,
                category: TldCategory::Premium,
                description: format!("Premium 2-letter TLD - {}", tld.to_uppercase()),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Reserved premium TLDs (×10 multiplier)
        let premium_tlds = vec![
            ("ai", TldCategory::Tech, "Artificial Intelligence domains"),
            ("m", TldCategory::Premium, "Mobile and premium domains"),
        ];
        
        for (tld, category, desc) in premium_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 10,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // IoT TLDs (×2 multiplier)
        let iot_tlds = vec![
            ("iot", TldCategory::IoT, "Internet of Things domains"),
            ("m2m", TldCategory::IoT, "Machine-to-Machine communication"),
        ];
        
        for (tld, category, desc) in iot_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 2,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Tech TLDs (×5 multiplier)
        let tech_tlds = vec![
            ("dlt", TldCategory::Tech, "Distributed Ledger Technology"),
            ("dag", TldCategory::Tech, "Directed Acyclic Graph"),
            ("aii", TldCategory::Tech, "Advanced AI Infrastructure"),
            ("def", TldCategory::Tech, "DeFi and decentralized finance"),
            ("dex", TldCategory::Tech, "Decentralized Exchange"),
            ("dht", TldCategory::Tech, "Distributed Hash Table"),
            ("vmn", TldCategory::Tech, "Virtual Machine Network"),
            ("nft", TldCategory::Tech, "Non-Fungible Tokens"),
            ("hsh", TldCategory::Tech, "Hash and cryptography"),
            ("ztk", TldCategory::Tech, "Zero-Knowledge Technology"),
            ("zkp", TldCategory::Tech, "Zero-Knowledge Proofs"),
            ("stg", TldCategory::Tech, "Smart Technology Group"),
            ("bft", TldCategory::Tech, "Byzantine Fault Tolerance"),
            ("lpk", TldCategory::Tech, "Lightweight Public Key"),
            ("p2p", TldCategory::Tech, "Peer-to-Peer networks"),
            ("sig", TldCategory::Tech, "Digital signatures"),
            ("ecd", TldCategory::Tech, "Elliptic Curve Digital"),
            ("edg", TldCategory::Tech, "Edge computing"),
        ];
        
        for (tld, category, desc) in tech_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 5,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Finance TLDs (×3 multiplier)
        let finance_tlds = vec![
            ("fin", TldCategory::Finance, "Financial services"),
            ("pay", TldCategory::Finance, "Payment processing"),
            ("fund", TldCategory::Finance, "Investment funds"),
            ("trx", TldCategory::Finance, "Transactions"),
        ];
        
        for (tld, category, desc) in finance_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 3,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Decentralized TLDs (×3 multiplier)
        let decentralized_tlds = vec![
            ("dao", TldCategory::Decentralized, "Decentralized Autonomous Organization"),
            ("dapp", TldCategory::Decentralized, "Decentralized Applications"),
            ("defi", TldCategory::Decentralized, "Decentralized Finance"),
        ];
        
        for (tld, category, desc) in decentralized_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 3,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Storage TLDs (×2 multiplier)
        let storage_tlds = vec![
            ("stor", TldCategory::Storage, "Storage services"),
            ("data", TldCategory::Storage, "Data management"),
            ("file", TldCategory::Storage, "File storage"),
            ("hash", TldCategory::Storage, "Content-addressed storage"),
        ];
        
        for (tld, category, desc) in storage_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 2,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Development TLDs (×2 multiplier)
        let dev_tlds = vec![
            ("dev", TldCategory::Development, "Development environments"),
            ("sdk", TldCategory::Development, "Software Development Kit"),
            ("api", TldCategory::Development, "Application Programming Interface"),
            ("lib", TldCategory::Development, "Libraries and frameworks"),
            ("core", TldCategory::Development, "Core infrastructure"),
        ];
        
        for (tld, category, desc) in dev_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 2,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Standard TLDs (×1 multiplier)
        let standard_tlds = vec![
            ("app", TldCategory::Standard, "Applications"),
            ("web", TldCategory::Standard, "Web services"),
            ("net", TldCategory::Standard, "Network services"),
            ("org", TldCategory::Standard, "Organizations"),
            ("com", TldCategory::Standard, "Commercial services"),
            ("info", TldCategory::Standard, "Information services"),
            ("blog", TldCategory::Standard, "Blogging platforms"),
            ("shop", TldCategory::Standard, "E-commerce"),
            ("news", TldCategory::Standard, "News and media"),
            ("edu", TldCategory::Standard, "Education"),
            ("gov", TldCategory::Standard, "Government services"),
            ("mil", TldCategory::Standard, "Military services"),
            ("int", TldCategory::Standard, "International organizations"),
        ];
        
        for (tld, category, desc) in standard_tlds {
            self.tlds.insert(tld.to_string(), TldEntry {
                tld: tld.to_string(),
                premium_multiplier: 1,
                category: category.clone(),
                description: desc.to_string(),
                available: true,
                min_years: 1,
                max_years: 20,
            });
        }
        
        // Default IPPAN TLD
        self.tlds.insert("ipn".to_string(), TldEntry {
            tld: "ipn".to_string(),
            premium_multiplier: 1,
            category: TldCategory::Standard,
            description: "Default IPPAN network domain".to_string(),
            available: true,
            min_years: 1,
            max_years: 20,
        });
    }
    
    /// Get TLD entry by name
    pub fn get_tld(&self, tld: &str) -> Option<&TldEntry> {
        self.tlds.get(tld)
    }
    
    /// Check if TLD is available
    pub fn is_tld_available(&self, tld: &str) -> bool {
        self.tlds.get(tld)
            .map(|entry| entry.available)
            .unwrap_or(false)
    }
    
    /// Get premium multiplier for TLD
    pub fn get_premium_multiplier(&self, tld: &str) -> u32 {
        self.tlds.get(tld)
            .map(|entry| entry.premium_multiplier)
            .unwrap_or(1)
    }
    
    /// List all available TLDs
    pub fn list_available_tlds(&self) -> Vec<&TldEntry> {
        self.tlds.values()
            .filter(|entry| entry.available)
            .collect()
    }
    
    /// List TLDs by category
    pub fn list_tlds_by_category(&self, category: &TldCategory) -> Vec<&TldEntry> {
        self.tlds.values()
            .filter(|entry| entry.available && entry.category == *category)
            .collect()
    }
    
    /// List TLDs by premium multiplier
    pub fn list_tlds_by_multiplier(&self, multiplier: u32) -> Vec<&TldEntry> {
        self.tlds.values()
            .filter(|entry| entry.available && entry.premium_multiplier == multiplier)
            .collect()
    }
    
    /// Get TLD statistics
    pub fn get_statistics(&self) -> TldStatistics {
        let mut stats = TldStatistics {
            total_tlds: 0,
            available_tlds: 0,
            by_category: HashMap::new(),
            by_multiplier: HashMap::new(),
        };
        
        for entry in self.tlds.values() {
            stats.total_tlds += 1;
            if entry.available {
                stats.available_tlds += 1;
                
                *stats.by_category.entry(entry.category.clone()).or_insert(0) += 1;
                *stats.by_multiplier.entry(entry.premium_multiplier).or_insert(0) += 1;
            }
        }
        
        stats
    }
    
    /// Export registry to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Import registry from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// TLD registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TldStatistics {
    /// Total number of TLDs
    pub total_tlds: usize,
    /// Number of available TLDs
    pub available_tlds: usize,
    /// Count by category
    pub by_category: HashMap<TldCategory, usize>,
    /// Count by premium multiplier
    pub by_multiplier: HashMap<u32, usize>,
}

impl Default for TldRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tld_registry_creation() {
        let registry = TldRegistry::new();
        
        // Check that default TLD exists
        assert!(registry.tlds.contains_key("ipn"));
        assert_eq!(registry.get_premium_multiplier("ipn"), 1);
        
        // Check premium TLDs
        assert_eq!(registry.get_premium_multiplier("ai"), 10);
        assert_eq!(registry.get_premium_multiplier("m"), 10);
        
        // Check IoT TLDs
        assert_eq!(registry.get_premium_multiplier("iot"), 2);
        assert_eq!(registry.get_premium_multiplier("m2m"), 2);
        
        // Check tech TLDs
        assert_eq!(registry.get_premium_multiplier("dlt"), 5);
        assert_eq!(registry.get_premium_multiplier("nft"), 5);
    }
    
    #[test]
    fn test_tld_availability() {
        let registry = TldRegistry::new();
        
        assert!(registry.is_tld_available("ipn"));
        assert!(registry.is_tld_available("ai"));
        assert!(registry.is_tld_available("iot"));
        assert!(!registry.is_tld_available("nonexistent"));
    }
    
    #[test]
    fn test_tld_categories() {
        let registry = TldRegistry::new();
        
        let standard_tlds = registry.list_tlds_by_category(&TldCategory::Standard);
        let premium_tlds = registry.list_tlds_by_category(&TldCategory::Premium);
        let tech_tlds = registry.list_tlds_by_category(&TldCategory::Tech);
        
        assert!(!standard_tlds.is_empty());
        assert!(!premium_tlds.is_empty());
        assert!(!tech_tlds.is_empty());
    }
    
    #[test]
    fn test_tld_statistics() {
        let registry = TldRegistry::new();
        let stats = registry.get_statistics();
        
        assert!(stats.total_tlds > 0);
        assert!(stats.available_tlds > 0);
        assert!(stats.available_tlds <= stats.total_tlds);
        assert!(!stats.by_category.is_empty());
        assert!(!stats.by_multiplier.is_empty());
    }
    
    #[test]
    fn test_json_serialization() {
        let registry = TldRegistry::new();
        
        let json = registry.to_json().unwrap();
        let deserialized = TldRegistry::from_json(&json).unwrap();
        
        assert_eq!(registry.tlds.len(), deserialized.tlds.len());
        assert_eq!(registry.get_premium_multiplier("ipn"), deserialized.get_premium_multiplier("ipn"));
    }
}
