//! DNS resolver for IPPAN on-chain DNS system

use super::types::*;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// DNS zone resolver
pub struct ZoneResolver {
    /// Zone storage
    zones: Arc<RwLock<BTreeMap<String, Zone>>>,
}

impl ZoneResolver {
    /// Create a new zone resolver
    pub fn new(zones: Arc<RwLock<BTreeMap<String, Zone>>>) -> Self {
        Self { zones }
    }

    /// Resolve a DNS query
    pub async fn resolve(&self, name: &str, rtype: Rtype) -> Result<Option<Rrset>> {
        // Handle IPPAN naming convention: ipn.domain.tld
        if let Some(stripped_name) = self.strip_ipn_prefix(name) {
            return self.resolve_internal(stripped_name, rtype).await;
        }
        
        // Standard DNS resolution
        self.resolve_internal(name, rtype).await
    }

    /// Internal resolution method (without IPPAN prefix handling)
    async fn resolve_internal(&self, name: &str, rtype: Rtype) -> Result<Option<Rrset>> {
        let (domain, label) = self.parse_query_name(name)?;
        
        let zones = self.zones.read().await;
        let zone = zones.get(&domain)
            .ok_or_else(|| anyhow::anyhow!("Zone not found: {}", domain))?;
        
        // Look up the record set
        let key = RrsetKey {
            name: label.to_string(),
            rtype,
        };
        
        Ok(zone.rrsets.get(&key).cloned())
    }

    /// Handle IPPAN naming convention: strip "ipn." prefix if present
    /// 
    /// Examples:
    /// - "ipn.alice.ipn" -> "alice.ipn"
    /// - "ipn.dao.fin" -> "dao.fin"
    /// - "alice.ipn" -> None (no prefix to strip)
    fn strip_ipn_prefix<'a>(&self, name: &'a str) -> Option<&'a str> {
        if name.starts_with("ipn.") {
            Some(&name[4..]) // Remove "ipn." prefix
        } else {
            None
        }
    }

    /// Check if a name follows the IPPAN naming convention
    pub fn is_ipn_name(&self, name: &str) -> bool {
        name.starts_with("ipn.")
    }

    /// Resolve an IPPAN name (with ipn. prefix handling)
    pub async fn resolve_ipn_name(&self, name: &str, rtype: Rtype) -> Result<Option<Rrset>> {
        if !self.is_ipn_name(name) {
            return Err(crate::IppanError::Validation(format!("Not an IPPAN name: {}", name)));
        }
        
        let stripped_name = self.strip_ipn_prefix(name).unwrap();
        self.resolve_internal(stripped_name, rtype).await
    }

    /// Resolve a DNS query with full response
    pub async fn resolve_with_response(&self, name: &str, rtype: Rtype) -> Result<DnsResponse> {
        // Handle IPPAN naming convention: ipn.domain.tld
        let resolved_name = if let Some(stripped_name) = self.strip_ipn_prefix(name) {
            stripped_name
        } else {
            name
        };
        
        let (domain, label) = self.parse_query_name(resolved_name)?;
        
        let zones = self.zones.read().await;
        let zone = zones.get(&domain);
        
        match zone {
            Some(zone) => {
                let key = RrsetKey {
                    name: label.to_string(),
                    rtype: rtype.clone(),
                };
                
                let rrset = zone.rrsets.get(&key).cloned();
                
                if rrset.is_some() {
                    Ok(DnsResponse {
                        name: name.to_string(), // Keep original name for response
                        rtype,
                        rrset,
                        rcode: ResponseCode::NoError,
                        authority: None,
                    })
                } else {
                    // Return NXDOMAIN with authority records
                    let authority = self.get_authority_records(zone)?;
                    Ok(DnsResponse {
                        name: name.to_string(),
                        rtype,
                        rrset: None,
                        rcode: ResponseCode::NXDomain,
                        authority: Some(authority),
                    })
                }
            }
            None => {
                // Zone not found
                Ok(DnsResponse {
                    name: name.to_string(),
                    rtype,
                    rrset: None,
                    rcode: ResponseCode::NXDomain,
                    authority: None,
                })
            }
        }
    }

    /// Get all records for a domain
    pub async fn get_zone_records(&self, domain: &str) -> Result<Option<Vec<(RrsetKey, Rrset)>>> {
        let zones = self.zones.read().await;
        let zone = zones.get(domain);
        
        match zone {
            Some(zone) => {
                let records: Vec<(RrsetKey, Rrset)> = zone.rrsets.clone().into_iter().collect();
                Ok(Some(records))
            }
            None => Ok(None),
        }
    }

    /// Search for records by pattern
    pub async fn search_records(&self, domain: &str, pattern: &str) -> Result<Vec<(RrsetKey, Rrset)>> {
        let zones = self.zones.read().await;
        let zone = zones.get(domain)
            .ok_or_else(|| anyhow::anyhow!("Zone not found: {}", domain))?;
        
        let mut results = Vec::new();
        
        for (key, rrset) in &zone.rrsets {
            if key.name.contains(pattern) {
                results.push((key.clone(), rrset.clone()));
            }
        }
        
        Ok(results)
    }

    /// Get authority records for a zone
    fn get_authority_records(&self, zone: &Zone) -> Result<Rrset> {
        // Look for NS records at apex
        let ns_key = RrsetKey {
            name: "@".to_string(),
            rtype: Rtype::NS,
        };
        
        if let Some(ns_rrset) = zone.rrsets.get(&ns_key) {
            return Ok(ns_rrset.clone());
        }
        
        // If no NS records, return empty authority
        Ok(Rrset {
            ttl: 3600,
            records: Vec::new(),
        })
    }

    /// Parse a query name into domain and label
    fn parse_query_name(&self, name: &str) -> Result<(String, String)> {
        // Handle apex queries
        if name.ends_with('.') {
            let domain = name.trim_end_matches('.');
            return Ok((domain.to_string(), "@".to_string()));
        }
        
        // Handle subdomain queries - find the second-to-last dot for proper domain parsing
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() >= 2 {
            // For names like "www.example.ipn", we want domain="example.ipn", label="www"
            let domain = parts[1..].join(".");
            let label = parts[0];
            
            return Ok((domain, label.to_string()));
        } else if parts.len() == 1 {
            // Single label query (assume apex)
            return Ok((parts[0].to_string(), "@".to_string()));
        }
        
        // Fallback for edge cases
        if let Some(dot_pos) = name.rfind('.') {
            let domain = &name[dot_pos + 1..];
            let label = &name[..dot_pos];
            
            // Handle apex with subdomain
            if label.is_empty() {
                return Ok((domain.to_string(), "@".to_string()));
            }
            
            return Ok((domain.to_string(), label.to_string()));
        }
        
        // Single label query (assume apex)
        Ok((name.to_string(), "@".to_string()))
    }

    /// Check if a domain exists
    pub async fn domain_exists(&self, domain: &str) -> bool {
        let zones = self.zones.read().await;
        zones.contains_key(domain)
    }

    /// Get zone statistics
    pub async fn get_zone_stats(&self, domain: &str) -> Result<Option<ZoneStats>> {
        let zones = self.zones.read().await;
        let zone = zones.get(domain);
        
        match zone {
            Some(zone) => {
                let mut stats = ZoneStats {
                    domain: domain.to_string(),
                    record_count: zone.rrsets.len(),
                    record_types: BTreeMap::new(),
                    last_updated: zone.updated_at_us,
                    serial: zone.serial,
                };
                
                // Count record types
                for (key, _) in &zone.rrsets {
                    *stats.record_types.entry(key.rtype.clone()).or_insert(0) += 1;
                }
                
                Ok(Some(stats))
            }
            None => Ok(None),
        }
    }

    /// Get all zones
    pub async fn get_all_zones(&self) -> Vec<String> {
        let zones = self.zones.read().await;
        zones.keys().cloned().collect()
    }

    /// Get zones by owner
    pub async fn get_zones_by_owner(&self, owner_pk: &[u8; 32]) -> Vec<String> {
        let zones = self.zones.read().await;
        let mut owned_zones = Vec::new();
        
        for (domain, zone) in zones.iter() {
            if zone.owner_pk == *owner_pk {
                owned_zones.push(domain.clone());
            }
        }
        
        owned_zones
    }
}

/// Zone statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneStats {
    /// Domain name
    pub domain: String,
    /// Total number of records
    pub record_count: usize,
    /// Record type counts
    pub record_types: BTreeMap<Rtype, usize>,
    /// Last update timestamp
    pub last_updated: u64,
    /// Current serial number
    pub serial: u32,
}

/// DNS query cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Query name
    pub name: String,
    /// Query type
    pub rtype: Rtype,
    /// Cached response
    pub response: DnsResponse,
    /// Cache expiry timestamp
    pub expires_at: u64,
}

/// DNS query cache
pub struct QueryCache {
    /// Cache entries
    entries: BTreeMap<String, CacheEntry>,
    /// Maximum cache size
    max_size: usize,
}

impl QueryCache {
    /// Create a new query cache
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_size,
        }
    }

    /// Get a cached response
    pub fn get(&self, name: &str, rtype: &Rtype) -> Option<&DnsResponse> {
        let key = self.make_cache_key(name, rtype);
        let entry = self.entries.get(&key)?;
        
        // Check if entry has expired
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u64;
        
        if now > entry.expires_at {
            return None;
        }
        
        Some(&entry.response)
    }

    /// Cache a response
    pub fn put(&mut self, name: &str, rtype: &Rtype, response: DnsResponse, ttl: u32) {
        let key = self.make_cache_key(name, rtype);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u64;
        
        let entry = CacheEntry {
            name: name.to_string(),
            rtype: rtype.clone(),
            response,
            expires_at: now + ttl as u64,
        };
        
        // Evict expired entries first
        self.evict_expired();
        
        // Evict oldest entries if cache is full
        if self.entries.len() >= self.max_size {
            self.evict_oldest();
        }
        
        self.entries.insert(key, entry);
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u64;
        
        let mut expired_count = 0;
        for entry in self.entries.values() {
            if now > entry.expires_at {
                expired_count += 1;
            }
        }
        
        CacheStats {
            total_entries: self.entries.len(),
            expired_entries: expired_count,
            max_size: self.max_size,
        }
    }

    /// Make cache key
    fn make_cache_key(&self, name: &str, rtype: &Rtype) -> String {
        format!("{}:{}", name, rtype)
    }

    /// Evict expired entries
    fn evict_expired(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u64;
        
        self.entries.retain(|_, entry| entry.expires_at > now);
    }

    /// Evict oldest entries
    fn evict_oldest(&mut self) {
        let target_size = self.max_size / 2; // Evict half when full
        
        while self.entries.len() > target_size {
            if let Some((key, _)) = self.entries.iter().next() {
                let key = key.clone();
                self.entries.remove(&key);
            } else {
                break;
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of entries
    pub total_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
    /// Maximum cache size
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_name() {
        let resolver = ZoneResolver::new(Arc::new(RwLock::new(BTreeMap::new())));
        
        // Test apex query
        let (domain, label) = resolver.parse_query_name("example.ipn.").unwrap();
        assert_eq!(domain, "example.ipn");
        assert_eq!(label, "@");
        
        // Test subdomain query
        let (domain, label) = resolver.parse_query_name("www.example.ipn").unwrap();
        assert_eq!(domain, "example.ipn");
        assert_eq!(label, "www");
        
        // Test single label
        let (domain, label) = resolver.parse_query_name("example").unwrap();
        assert_eq!(domain, "example");
        assert_eq!(label, "@");
    }

    #[test]
    fn test_query_cache() {
        let mut cache = QueryCache::new(100);
        
        // Test cache put/get
        let response = DnsResponse {
            name: "www.example.ipn".to_string(),
            rtype: Rtype::A,
            rrset: Some(Rrset {
                ttl: 300,
                records: vec![serde_json::json!("192.168.1.1")],
            }),
            rcode: ResponseCode::NoError,
            authority: None,
        };
        
        cache.put("www.example.ipn", &Rtype::A, response.clone(), 300);
        
        let cached = cache.get("www.example.ipn", &Rtype::A);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().name, "www.example.ipn");
        
        // Test cache stats
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }
}
