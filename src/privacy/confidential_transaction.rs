//! Confidential Transaction Framework for IPPAN
//! 
//! This module implements privacy-preserving transactions where details are
//! accessible only to entitled parties (payer, receiver, authorized entities).

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use rand::RngCore;
use sha2::{Sha256, Digest};

/// Transaction hash type
pub type TransactionHash = [u8; 32];

/// Confidential transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialTransaction {
    /// Transaction hash (public)
    pub hash: TransactionHash,
    /// Encrypted transaction data
    pub encrypted_data: EncryptedTransactionData,
    /// Public metadata (for routing/validation)
    pub public_metadata: PublicMetadata,
    /// Access control list
    pub access_control: AccessControlList,
    /// Zero-knowledge proof of validity
    pub validity_proof: ZKProof,
    /// Transaction signature
    pub signature: Option<Vec<u8>>,
}

/// Encrypted transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedTransactionData {
    /// Encrypted payload (AES-256-GCM)
    pub ciphertext: Vec<u8>,
    /// Key encapsulation (using recipient's public key)
    pub key_encapsulation: Vec<u8>,
    /// Nonce for encryption
    pub nonce: Vec<u8>,
    /// Authentication tag
    pub tag: Vec<u8>,
    /// Encryption timestamp
    pub encrypted_at: DateTime<Utc>,
}

/// Public metadata (visible to all nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicMetadata {
    /// Transaction type (payment, staking, etc.)
    pub transaction_type: TransactionType,
    /// Sender address (for routing)
    pub sender_address: String,
    /// Recipient address (for routing)
    pub recipient_address: String,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Fee amount (public for economic reasons)
    pub fee: u64,
    /// Transaction status
    pub status: TransactionStatus,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
}

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Payment transaction
    Payment,
    /// Staking transaction
    Staking,
    /// Storage transaction
    Storage,
    /// Domain transaction
    Domain,
    /// M2M payment transaction
    M2MPayment,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Pending
    Pending,
    /// Confirmed
    Confirmed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}

/// Privacy levels for transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Public transaction (current behavior)
    Public,
    /// Confidential transaction (amount hidden)
    Confidential,
    /// Private transaction (all details hidden)
    Private,
    /// Regulated transaction (with compliance access)
    Regulated,
}

/// Access control list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlList {
    /// Authorized parties (addresses)
    pub authorized_parties: Vec<String>,
    /// Access permissions
    pub permissions: Vec<Permission>,
    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Time-based access control
    pub time_based_access: Option<TimeBasedAccess>,
    /// Attribute-based access control
    pub attribute_based_access: Option<AttributeBasedAccess>,
}

/// Access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    /// Can read transaction data
    Read,
    /// Can decrypt transaction data
    Decrypt,
    /// Can modify access control
    ModifyAccess,
    /// Can revoke access
    RevokeAccess,
    /// Can audit transaction
    Audit,
    /// Can share with others
    Share,
}

/// Time-based access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedAccess {
    /// Access granted from
    pub access_from: DateTime<Utc>,
    /// Access granted until
    pub access_until: DateTime<Utc>,
    /// Maximum number of accesses
    pub max_accesses: Option<u32>,
    /// Current access count
    pub current_accesses: u32,
    /// Access history
    pub access_history: Vec<AccessRecord>,
}

/// Access record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRecord {
    /// Access timestamp
    pub timestamp: DateTime<Utc>,
    /// Accessing party
    pub accessing_party: String,
    /// Access type (read, decrypt, etc.)
    pub access_type: AccessType,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
}

/// Access type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessType {
    /// Read access
    Read,
    /// Decrypt access
    Decrypt,
    /// Audit access
    Audit,
    /// Share access
    Share,
}

/// Attribute-based access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBasedAccess {
    /// Required attributes for access
    pub required_attributes: Vec<Attribute>,
    /// Attribute authorities
    pub attribute_authorities: Vec<String>,
    /// Access policy
    pub access_policy: AccessPolicy,
}

/// Access attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Attribute {
    /// User is sender
    IsSender,
    /// User is recipient
    IsRecipient,
    /// User is regulator
    IsRegulator,
    /// User is auditor
    IsAuditor,
    /// User has specific role
    HasRole(String),
    /// User has specific permission
    HasPermission(String),
    /// Transaction amount threshold
    AmountThreshold(u64),
    /// Geographic region
    GeographicRegion(String),
    /// Time-based attribute
    TimeBased(DateTime<Utc>, DateTime<Utc>),
}

/// Access policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessPolicy {
    /// Allow access if ALL attributes are satisfied
    All(Vec<Attribute>),
    /// Allow access if ANY attributes are satisfied
    Any(Vec<Attribute>),
    /// Allow access if AT LEAST N attributes are satisfied
    AtLeast(usize, Vec<Attribute>),
    /// Custom policy
    Custom(String),
}

/// Zero-knowledge proof for transaction validity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKProof {
    /// Proof type
    pub proof_type: ZKProofType,
    /// Proof data
    pub proof_data: Vec<u8>,
    /// Public inputs
    pub public_inputs: Vec<Vec<u8>>,
    /// Verification key
    pub verification_key: Vec<u8>,
    /// Proof timestamp
    pub generated_at: DateTime<Utc>,
}

/// ZK proof types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZKProofType {
    /// Range proof for confidential amounts
    RangeProof,
    /// Balance proof (sender has sufficient funds)
    BalanceProof,
    /// Consistency proof (no double-spending)
    ConsistencyProof,
    /// Compliance proof (regulatory requirements)
    ComplianceProof,
    /// Aggregated proof (multiple proofs combined)
    AggregatedProof,
}

/// Confidential transaction manager
pub struct ConfidentialTransactionManager {
    /// Encryption keys
    keys: HashMap<String, Vec<u8>>,
    /// Access logs
    access_logs: Vec<AccessRecord>,
    /// Privacy settings
    privacy_settings: PrivacySettings,
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Default privacy level
    pub default_privacy_level: PrivacyLevel,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Enable access control
    pub enable_access_control: bool,
    /// Enable time-based access
    pub enable_time_based_access: bool,
    /// Enable attribute-based access
    pub enable_attribute_based_access: bool,
    /// Maximum access attempts
    pub max_access_attempts: u32,
    /// Access attempt window (seconds)
    pub access_attempt_window: u64,
}

impl ConfidentialTransactionManager {
    /// Create a new confidential transaction manager
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            access_logs: Vec::new(),
            privacy_settings: PrivacySettings {
                default_privacy_level: PrivacyLevel::Confidential,
                enable_audit_logging: true,
                enable_access_control: true,
                enable_time_based_access: true,
                enable_attribute_based_access: true,
                max_access_attempts: 5,
                access_attempt_window: 3600, // 1 hour
            },
        }
    }

    /// Create a confidential payment transaction
    pub async fn create_confidential_payment(
        &mut self,
        sender_address: String,
        recipient_address: String,
        amount: u64,
        fee: u64,
        memo: Option<String>,
        privacy_level: PrivacyLevel,
    ) -> Result<ConfidentialTransaction> {
        // Generate transaction hash
        let hash_data = format!("{}:{}:{}:{}:{}", 
            sender_address, recipient_address, amount, fee, Utc::now().timestamp());
        let hash = self.calculate_hash(hash_data.as_bytes());

        // Create transaction data
        let transaction_data = PaymentTransactionData {
            sender_address: sender_address.clone(),
            recipient_address: recipient_address.clone(),
            amount,
            memo,
            timestamp: Utc::now(),
        };

        // Encrypt transaction data
        let encrypted_data = self.encrypt_transaction_data(&transaction_data, &recipient_address).await?;

        // Create public metadata
        let public_metadata = PublicMetadata {
            transaction_type: TransactionType::Payment,
            sender_address,
            recipient_address,
            timestamp: Utc::now(),
            fee,
            status: TransactionStatus::Pending,
            privacy_level,
        };

        // Create access control list
        let access_control = self.create_access_control_list(
            &transaction_data.sender_address,
            &transaction_data.recipient_address,
            privacy_level,
        ).await?;

        // Generate zero-knowledge proof
        let validity_proof = self.generate_validity_proof(&transaction_data, &hash).await?;

        // Create confidential transaction
        let confidential_tx = ConfidentialTransaction {
            hash,
            encrypted_data,
            public_metadata,
            access_control,
            validity_proof,
            signature: None,
        };

        Ok(confidential_tx)
    }

    /// Decrypt transaction data (only for authorized parties)
    pub async fn decrypt_transaction_data(
        &mut self,
        confidential_tx: &ConfidentialTransaction,
        requesting_party: &str,
        access_type: AccessType,
    ) -> Result<PaymentTransactionData> {
        // Check access permissions
        if !self.check_access_permissions(confidential_tx, requesting_party, &access_type).await? {
            return Err(crate::error::IppanError::Validation(
                "Access denied: insufficient permissions".to_string()
            ));
        }

        // Log access attempt
        self.log_access_attempt(confidential_tx, requesting_party, &access_type).await?;

        // Decrypt transaction data
        let decrypted_data = self.decrypt_data(&confidential_tx.encrypted_data).await?;

        // Deserialize transaction data
        let transaction_data: PaymentTransactionData = serde_json::from_slice(&decrypted_data)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Failed to deserialize transaction data: {}", e)
            ))?;

        Ok(transaction_data)
    }

    /// Check access permissions
    async fn check_access_permissions(
        &self,
        confidential_tx: &ConfidentialTransaction,
        requesting_party: &str,
        access_type: &AccessType,
    ) -> Result<bool> {
        let access_control = &confidential_tx.access_control;

        // Check if party is authorized
        if !access_control.authorized_parties.contains(&requesting_party.to_string()) {
            return Ok(false);
        }

        // Check time-based access
        if let Some(time_based) = &access_control.time_based_access {
            let now = Utc::now();
            if now < time_based.access_from || now > time_based.access_until {
                return Ok(false);
            }

            if let Some(max_accesses) = time_based.max_accesses {
                if time_based.current_accesses >= max_accesses {
                    return Ok(false);
                }
            }
        }

        // Check attribute-based access
        if let Some(attribute_based) = &access_control.attribute_based_access {
            if !self.check_attribute_policy(attribute_based, requesting_party).await? {
                return Ok(false);
            }
        }

        // Check permissions
        let required_permission = match access_type {
            AccessType::Read => Permission::Read,
            AccessType::Decrypt => Permission::Decrypt,
            AccessType::Audit => Permission::Audit,
            AccessType::Share => Permission::Share,
        };

        if !access_control.permissions.contains(&required_permission) {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check attribute-based access policy
    async fn check_attribute_policy(
        &self,
        attribute_based: &AttributeBasedAccess,
        requesting_party: &str,
    ) -> Result<bool> {
        // This is a simplified implementation
        // In a real system, you would check against actual attribute authorities
        
        match &attribute_based.access_policy {
            AccessPolicy::All(attributes) => {
                for attribute in attributes {
                    if !self.check_attribute(attribute, requesting_party).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            AccessPolicy::Any(attributes) => {
                for attribute in attributes {
                    if self.check_attribute(attribute, requesting_party).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            AccessPolicy::AtLeast(n, attributes) => {
                let mut satisfied_count = 0;
                for attribute in attributes {
                    if self.check_attribute(attribute, requesting_party).await? {
                        satisfied_count += 1;
                    }
                }
                Ok(satisfied_count >= *n)
            }
            AccessPolicy::Custom(_policy) => {
                // Custom policy implementation would go here
                Ok(true)
            }
        }
    }

    /// Check individual attribute
    async fn check_attribute(
        &self,
        attribute: &Attribute,
        requesting_party: &str,
    ) -> Result<bool> {
        match attribute {
            Attribute::IsSender => {
                // Check if requesting party is the sender
                Ok(true) // Simplified
            }
            Attribute::IsRecipient => {
                // Check if requesting party is the recipient
                Ok(true) // Simplified
            }
            Attribute::IsRegulator => {
                // Check if requesting party is a regulator
                Ok(requesting_party.starts_with("regulator_"))
            }
            Attribute::IsAuditor => {
                // Check if requesting party is an auditor
                Ok(requesting_party.starts_with("auditor_"))
            }
            Attribute::HasRole(role) => {
                // Check if requesting party has the specified role
                Ok(requesting_party.contains(role))
            }
            Attribute::HasPermission(permission) => {
                // Check if requesting party has the specified permission
                Ok(requesting_party.contains(permission))
            }
            Attribute::AmountThreshold(threshold) => {
                // Check if transaction amount meets threshold
                Ok(true) // Simplified - would check actual amount
            }
            Attribute::GeographicRegion(region) => {
                // Check if requesting party is in the specified region
                Ok(true) // Simplified
            }
            Attribute::TimeBased(from, until) => {
                let now = Utc::now();
                Ok(now >= *from && now <= *until)
            }
        }
    }

    /// Create access control list
    async fn create_access_control_list(
        &self,
        sender: &str,
        recipient: &str,
        privacy_level: PrivacyLevel,
    ) -> Result<AccessControlList> {
        let mut authorized_parties = vec![sender.to_string(), recipient.to_string()];
        let mut permissions = vec![Permission::Read, Permission::Decrypt];

        // Add regulatory access for regulated transactions
        if matches!(privacy_level, PrivacyLevel::Regulated) {
            authorized_parties.push("regulator_global".to_string());
            permissions.push(Permission::Audit);
        }

        // Create time-based access control
        let time_based_access = if self.privacy_settings.enable_time_based_access {
            Some(TimeBasedAccess {
                access_from: Utc::now(),
                access_until: Utc::now() + chrono::Duration::days(365), // 1 year
                max_accesses: Some(1000),
                current_accesses: 0,
                access_history: Vec::new(),
            })
        } else {
            None
        };

        // Create attribute-based access control
        let attribute_based_access = if self.privacy_settings.enable_attribute_based_access {
            Some(AttributeBasedAccess {
                required_attributes: vec![
                    Attribute::IsSender,
                    Attribute::IsRecipient,
                ],
                attribute_authorities: vec!["ippan_authority".to_string()],
                access_policy: AccessPolicy::Any(vec![
                    Attribute::IsSender,
                    Attribute::IsRecipient,
                ]),
            })
        } else {
            None
        };

        Ok(AccessControlList {
            authorized_parties,
            permissions,
            expires_at: Some(Utc::now() + chrono::Duration::days(365)),
            time_based_access,
            attribute_based_access,
        })
    }

    /// Encrypt transaction data
    async fn encrypt_transaction_data(
        &self,
        transaction_data: &PaymentTransactionData,
        recipient_address: &str,
    ) -> Result<EncryptedTransactionData> {
        // Serialize transaction data
        let data_bytes = serde_json::to_vec(transaction_data)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Failed to serialize transaction data: {}", e)
            ))?;

        // Generate encryption key
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);

        // Encrypt data with AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Invalid encryption key: {}", e)
            ))?;

        // Generate nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt data
        let ciphertext = cipher.encrypt(nonce, &data_bytes)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Encryption failed: {}", e)
            ))?;

        // Split ciphertext into data and tag
        let tag_size = 16;
        let data_len = ciphertext.len() - tag_size;
        let encrypted_data = ciphertext[..data_len].to_vec();
        let tag = ciphertext[data_len..].to_vec();

        // Encrypt key for recipient (simplified - would use recipient's public key)
        let key_encapsulation = self.encrypt_key_for_recipient(&key, recipient_address).await?;

        Ok(EncryptedTransactionData {
            ciphertext: encrypted_data,
            key_encapsulation,
            nonce: nonce_bytes.to_vec(),
            tag,
            encrypted_at: Utc::now(),
        })
    }

    /// Encrypt key for recipient
    async fn encrypt_key_for_recipient(
        &self,
        key: &[u8; 32],
        recipient_address: &str,
    ) -> Result<Vec<u8>> {
        // This is a simplified implementation
        // In a real system, you would use the recipient's public key for encryption
        
        // For now, just return the key (in production, this would be encrypted)
        Ok(key.to_vec())
    }

    /// Decrypt data
    async fn decrypt_data(
        &self,
        encrypted_data: &EncryptedTransactionData,
    ) -> Result<Vec<u8>> {
        // Decrypt key (simplified)
        let key = &encrypted_data.key_encapsulation;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Invalid decryption key: {}", e)
            ))?;

        // Create nonce
        let nonce = Nonce::from_slice(&encrypted_data.nonce);

        // Combine encrypted data and tag
        let mut ciphertext = encrypted_data.ciphertext.clone();
        ciphertext.extend_from_slice(&encrypted_data.tag);

        // Decrypt data
        let decrypted_data = cipher.decrypt(nonce, &ciphertext)
            .map_err(|e| crate::error::IppanError::Validation(
                format!("Decryption failed: {}", e)
            ))?;

        Ok(decrypted_data)
    }

    /// Generate validity proof
    async fn generate_validity_proof(
        &self,
        transaction_data: &PaymentTransactionData,
        hash: &TransactionHash,
    ) -> Result<ZKProof> {
        // This is a simplified implementation
        // In a real system, you would generate actual zero-knowledge proofs
        
        let proof_data = format!("valid_proof_{:?}_{}", hash, transaction_data.timestamp.timestamp())
            .as_bytes()
            .to_vec();

        Ok(ZKProof {
            proof_type: ZKProofType::RangeProof,
            proof_data,
            public_inputs: vec![hash.to_vec()],
            verification_key: vec![1, 2, 3, 4], // Simplified
            generated_at: Utc::now(),
        })
    }

    /// Calculate hash
    fn calculate_hash(&self, data: &[u8]) -> TransactionHash {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Log access attempt
    async fn log_access_attempt(
        &mut self,
        confidential_tx: &ConfidentialTransaction,
        requesting_party: &str,
        access_type: &AccessType,
    ) -> Result<()> {
        if !self.privacy_settings.enable_audit_logging {
            return Ok(());
        }

        let access_record = AccessRecord {
            timestamp: Utc::now(),
            accessing_party: requesting_party.to_string(),
            access_type: access_type.clone(),
            ip_address: None, // Would be set in real implementation
            user_agent: None, // Would be set in real implementation
        };

        self.access_logs.push(access_record);

        // Update access count in time-based access control
        if let Some(time_based) = &mut confidential_tx.access_control.time_based_access {
            time_based.current_accesses += 1;
        }

        Ok(())
    }
}

/// Payment transaction data (encrypted in confidential transactions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTransactionData {
    /// Sender address
    pub sender_address: String,
    /// Recipient address
    pub recipient_address: String,
    /// Amount (confidential)
    pub amount: u64,
    /// Memo (confidential)
    pub memo: Option<String>,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_confidential_payment_creation() {
        let mut manager = ConfidentialTransactionManager::new();
        
        let confidential_tx = manager.create_confidential_payment(
            "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            "i1recipient1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            1000000, // 1 IPN
            100,     // 0.0001 IPN fee
            Some("Test payment".to_string()),
            PrivacyLevel::Confidential,
        ).await.unwrap();

        assert_eq!(confidential_tx.public_metadata.transaction_type, TransactionType::Payment);
        assert_eq!(confidential_tx.public_metadata.privacy_level, PrivacyLevel::Confidential);
        assert_eq!(confidential_tx.public_metadata.fee, 100);
    }

    #[tokio::test]
    async fn test_confidential_payment_decryption() {
        let mut manager = ConfidentialTransactionManager::new();
        
        let confidential_tx = manager.create_confidential_payment(
            "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            "i1recipient1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            1000000,
            100,
            Some("Test payment".to_string()),
            PrivacyLevel::Confidential,
        ).await.unwrap();

        // Decrypt as sender (should succeed)
        let decrypted_data = manager.decrypt_transaction_data(
            &confidential_tx,
            "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            AccessType::Decrypt,
        ).await.unwrap();

        assert_eq!(decrypted_data.amount, 1000000);
        assert_eq!(decrypted_data.memo, Some("Test payment".to_string()));

        // Decrypt as unauthorized party (should fail)
        let result = manager.decrypt_transaction_data(
            &confidential_tx,
            "i1unauthorized1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            AccessType::Decrypt,
        ).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_regulated_transaction() {
        let mut manager = ConfidentialTransactionManager::new();
        
        let confidential_tx = manager.create_confidential_payment(
            "i1sender1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            "i1recipient1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            1000000,
            100,
            Some("Test payment".to_string()),
            PrivacyLevel::Regulated,
        ).await.unwrap();

        // Check that regulator is in authorized parties
        assert!(confidential_tx.access_control.authorized_parties.contains(&"regulator_global".to_string()));
        assert!(confidential_tx.access_control.permissions.contains(&Permission::Audit));
    }
}
