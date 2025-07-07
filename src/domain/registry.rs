//! Domain registry module
//! 
//! Handles domain name registration and management

use crate::{Result, TransactionHash};
use super::{DomainInfo, DomainStatus, RenewalRecord};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use std::sync::Arc;
use super::{DomainRecord, DomainType};

/// Domain registry for managing domain registrations
pub struct DomainRegistry {
    /// Registered domains
    domains: Arc<RwLock<HashMap<String, DomainRecord>>>,
    /// Domain ownership mapping
    ownership: Arc<RwLock<HashMap<[u8; 32], Vec<String>>>>,
    /// Domain transaction history
    transactions: Arc<RwLock<Vec<DomainTransaction>>>,
    /// Domain expiration index
    expiration_index: Arc<RwLock<HashMap<u64, Vec<String>>>>,
}

/// Domain transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainTransaction {
    /// Transaction hash
    pub hash: TransactionHash,
    /// Domain name
    pub domain: String,
    /// Transaction type
    pub tx_type: DomainTxType,
    /// Owner public key
    pub owner: [u8; 32],
    /// Fee paid
    pub fee: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Block height (if confirmed)
    pub block_height: Option<u64>,
}

/// Domain transaction types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DomainTxType {
    /// Register domain
    Register,
    /// Renew domain
    Renew,
    /// Transfer domain
    Transfer,
    /// Delete domain
    Delete,
}

impl DomainRegistry {
    /// Create a new domain registry
    pub fn new() -> Self {
        Self {
            domains: Arc::new(RwLock::new(HashMap::new())),
            ownership: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(Vec::new())),
            expiration_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new domain
    pub async fn register_domain(&self, record: DomainRecord) -> Result<()> {
        let domain_name = record.name.clone();
        let owner = record.owner;
        let expires_at = record.expires_at;
        
        // Add to domains
        {
            let mut domains = self.domains.write().await;
            domains.insert(domain_name.clone(), record);
        }
        
        // Update ownership mapping
        {
            let mut ownership = self.ownership.write().await;
            ownership.entry(owner).or_insert_with(Vec::new).push(domain_name.clone());
        }
        
        // Update expiration index
        {
            let mut expiration = self.expiration_index.write().await;
            expiration.entry(expires_at).or_insert_with(Vec::new).push(domain_name);
        }
        
        Ok(())
    }

    /// Check if domain is registered
    pub async fn is_domain_registered(&self, name: &str) -> Result<bool> {
        let domains = self.domains.read().await;
        Ok(domains.contains_key(name))
    }

    /// Get domain information
    pub async fn get_domain_info(&self, name: &str) -> Result<DomainRecord> {
        let domains = self.domains.read().await;
        domains.get(name)
            .cloned()
            .ok_or_else(|| crate::IppanError::Domain("Domain not found".to_string()))
    }

    /// Transfer domain ownership
    pub async fn transfer_domain(&self, name: &str, current_owner: [u8; 32], new_owner: [u8; 32]) -> Result<TransactionHash> {
        let mut domains = self.domains.write().await;
        let domain_info = domains.get_mut(name)
            .ok_or_else(|| crate::IppanError::Domain("Domain not found".to_string()))?;

        if domain_info.owner != current_owner {
            return Err(crate::IppanError::Domain("Not the domain owner".to_string()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update ownership
        domain_info.owner = new_owner;

        // Update ownership mapping
        {
            let mut ownership = self.ownership.write().await;
            
            // Remove from current owner
            if let Some(domains_list) = ownership.get_mut(&current_owner) {
                domains_list.retain(|d| d != name);
            }

            // Add to new owner
            ownership.entry(new_owner).or_insert_with(Vec::new).push(name.to_string());
        }

        // Create transaction
        let tx_hash = self.create_transaction(
            name,
            DomainTxType::Transfer,
            new_owner,
            0, // No fee for transfers
            timestamp,
        ).await?;

        Ok(tx_hash)
    }

    /// Update domain data
    pub async fn update_domain_data(&self, name: &str, data: String) -> Result<()> {
        let mut domains = self.domains.write().await;
        let domain_info = domains.get_mut(name)
            .ok_or_else(|| crate::IppanError::Domain("Domain not found".to_string()))?;

        domain_info.data = Some(data);
        Ok(())
    }

    /// Search for domains
    pub async fn search_domains(&self, query: &str, limit: Option<usize>) -> Result<Vec<DomainRecord>> {
        let domains = self.domains.read().await;
        let mut results = Vec::new();

        for domain_info in domains.values() {
            if domain_info.name.contains(query) {
                results.push(domain_info.clone());
            }
        }

        // Sort by registration date (newest first)
        results.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));

        // Apply limit if specified
        if let Some(limit) = limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get domains owned by an address
    pub async fn get_domains_by_owner(&self, owner: [u8; 32]) -> Result<Vec<DomainRecord>> {
        let ownership = self.ownership.read().await;
        let domains = self.domains.read().await;
        let mut results = Vec::new();

        if let Some(domain_names) = ownership.get(&owner) {
            for name in domain_names {
                if let Some(domain_info) = domains.get(name) {
                    results.push(domain_info.clone());
                }
            }
        }

        // Sort by registration date (newest first)
        results.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));

        Ok(results)
    }

    /// Get total number of domains
    pub async fn get_total_domains(&self) -> Result<u64> {
        let domains = self.domains.read().await;
        Ok(domains.len() as u64)
    }

    /// Get number of active domains
    pub async fn get_active_domains(&self) -> Result<u64> {
        let domains = self.domains.read().await;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let active_count = domains.values()
            .filter(|domain| {
                matches!(domain.status, DomainStatus::Active) && domain.expires_at > current_time
            })
            .count();

        Ok(active_count as u64)
    }

    /// Get number of expired domains
    pub async fn get_expired_domains(&self) -> Result<u64> {
        let domains = self.domains.read().await;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expired_count = domains.values()
            .filter(|domain| domain.expires_at <= current_time)
            .count();

        Ok(expired_count as u64)
    }

    /// Get expiring domains
    pub async fn get_expiring_domains(&self, days: u64) -> Result<Vec<DomainRecord>> {
        let domains = self.domains.read().await;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff_time = current_time + (days * 24 * 60 * 60);

        let mut expiring = domains.values()
            .filter(|domain| {
                domain.expires_at <= cutoff_time && domain.expires_at > current_time
            })
            .cloned()
            .collect::<Vec<_>>();

        // Sort by expiration date (earliest first)
        expiring.sort_by(|a, b| a.expires_at.cmp(&b.expires_at));

        Ok(expiring)
    }

    /// Renew domain
    pub async fn renew_domain(&self, name: &str, owner: [u8; 32], fee: u64) -> Result<()> {
        let mut domains = self.domains.write().await;
        let domain_info = domains.get_mut(name)
            .ok_or_else(|| crate::IppanError::Domain("Domain not found".to_string()))?;

        if domain_info.owner != owner {
            return Err(crate::IppanError::Domain("Not the domain owner".to_string()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Extend expiration by 1 year
        let new_expires_at = if domain_info.expires_at > timestamp {
            domain_info.expires_at + (365 * 24 * 60 * 60)
        } else {
            timestamp + (365 * 24 * 60 * 60)
        };

        // Add renewal record
        let renewal_record = RenewalRecord {
            timestamp,
            fee,
            new_expires_at,
            tx_hash: [0; 32], // Will be set by caller
        };

        domain_info.renewals.push(renewal_record);
        domain_info.expires_at = new_expires_at;
        domain_info.status = DomainStatus::Active;

        Ok(())
    }

    /// Get domain transaction history
    pub async fn get_domain_transactions(&self, name: &str, limit: Option<usize>) -> Result<Vec<DomainTransaction>> {
        let transactions = self.transactions.read().await;
        let mut results = transactions.iter()
            .filter(|tx| tx.domain == name)
            .cloned()
            .collect::<Vec<_>>();

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit if specified
        if let Some(limit) = limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Create a domain transaction
    async fn create_transaction(
        &self,
        domain: &str,
        tx_type: DomainTxType,
        owner: [u8; 32],
        fee: u64,
        timestamp: u64,
    ) -> Result<TransactionHash> {
        let transaction = DomainTransaction {
            hash: [0; 32], // Will be calculated
            domain: domain.to_string(),
            tx_type,
            owner,
            fee,
            timestamp,
            block_height: None,
        };

        let tx_hash = self.calculate_transaction_hash(&transaction);
        let mut transaction = transaction;
        transaction.hash = tx_hash;

        // Add to transaction history
        {
            let mut transactions = self.transactions.write().await;
            transactions.push(transaction);
        }

        Ok(tx_hash)
    }

    /// Calculate transaction hash
    fn calculate_transaction_hash(&self, transaction: &DomainTransaction) -> TransactionHash {
        let mut hasher = Sha256::new();
        hasher.update(&transaction.domain);
        hasher.update(&(transaction.tx_type as u8).to_le_bytes());
        hasher.update(&transaction.owner);
        hasher.update(&transaction.fee.to_le_bytes());
        hasher.update(&transaction.timestamp.to_le_bytes());
        hasher.finalize().into()
    }

    /// Get domains expiring before timestamp
    pub async fn get_domains_expiring_before(&self, timestamp: u64) -> Vec<DomainRecord> {
        let expiration = self.expiration_index.read().await;
        let domains = self.domains.read().await;
        
        let mut expiring_domains = Vec::new();
        
        for (expires_at, domain_names) in expiration.iter() {
            if *expires_at <= timestamp {
                for name in domain_names {
                    if let Some(record) = domains.get(name) {
                        expiring_domains.push(record.clone());
                    }
                }
            }
        }
        
        expiring_domains
    }
    
    /// Get all domains
    pub async fn get_all_domains(&self) -> Vec<DomainRecord> {
        let domains = self.domains.read().await;
        domains.values().cloned().collect()
    }
    
    /// Get domain count
    pub async fn domain_count(&self) -> usize {
        let domains = self.domains.read().await;
        domains.len()
    }
    
    /// Check if domain exists
    pub async fn domain_exists(&self, name: &str) -> bool {
        let domains = self.domains.read().await;
        domains.contains_key(name)
    }
    
    /// Get domains by status
    pub async fn get_domains_by_status(&self, status: DomainStatus) -> Vec<DomainRecord> {
        let domains = self.domains.read().await;
        domains
            .values()
            .filter(|record| record.status == status)
            .cloned()
            .collect()
    }
    
    /// Get domains by type
    pub async fn get_domains_by_type(&self, domain_type: DomainType) -> Vec<DomainRecord> {
        let domains = self.domains.read().await;
        domains
            .values()
            .filter(|record| record.domain_type == domain_type)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_registration() {
        let registry = DomainRegistry::new();
        let owner = [1u8; 32];
        let domain_name = "test.ipn";
        let fee = 1_000_000_000;

        // Register domain
        let tx_hash = registry.register_domain(domain_name, owner, fee).await.unwrap();
        assert_ne!(tx_hash, [0u8; 32]);

        // Check if registered
        assert!(registry.is_domain_registered(domain_name).await.unwrap());

        // Get domain info
        let domain_info = registry.get_domain_info(domain_name).await.unwrap();
        assert_eq!(domain_info.owner, owner);
        assert_eq!(domain_info.name, domain_name);
    }

    #[tokio::test]
    async fn test_domain_transfer() {
        let registry = DomainRegistry::new();
        let owner1 = [1u8; 32];
        let owner2 = [2u8; 32];
        let domain_name = "transfer.ipn";
        let fee = 1_000_000_000;

        // Register domain
        registry.register_domain(domain_name, owner1, fee).await.unwrap();

        // Transfer domain
        let tx_hash = registry.transfer_domain(domain_name, owner1, owner2).await.unwrap();
        assert_ne!(tx_hash, [0u8; 32]);

        // Check new owner
        let domain_info = registry.get_domain_info(domain_name).await.unwrap();
        assert_eq!(domain_info.owner, owner2);
    }

    #[tokio::test]
    async fn test_domain_search() {
        let registry = DomainRegistry::new();
        let owner = [1u8; 32];
        let fee = 1_000_000_000;

        // Register multiple domains
        registry.register_domain("alice.ipn", owner, fee).await.unwrap();
        registry.register_domain("bob.ipn", owner, fee).await.unwrap();
        registry.register_domain("charlie.ipn", owner, fee).await.unwrap();

        // Search for domains
        let results = registry.search_domains("alice", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "alice.ipn");
    }
}
