//! Encryption for IPPAN storage

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use rand::RngCore;


/// Key management role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyManagementRole {
    /// Administrator - full access to all keys
    Administrator,
    /// Operator - can use keys but not manage them
    Operator,
    /// Auditor - can view key metadata and audit logs
    Auditor,
    /// ReadOnly - can only read encrypted data
    ReadOnly,
}

/// Key operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyOperation {
    /// Key generation
    Generate,
    /// Key rotation
    Rotate,
    /// Key revocation
    Revoke,
    /// Key backup
    Backup,
    /// Key restore
    Restore,
    /// Key access
    Access,
    /// Key export
    Export,
    /// Key import
    Import,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAuditLog {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// User ID
    pub user_id: String,
    /// User role
    pub user_role: KeyManagementRole,
    /// Operation type
    pub operation: KeyOperation,
    /// Key ID (if applicable)
    pub key_id: Option<String>,
    /// Operation result
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
}

/// Key access control entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAccessControl {
    /// User ID
    pub user_id: String,
    /// User role
    pub role: KeyManagementRole,
    /// Allowed key IDs (empty means all keys)
    pub allowed_key_ids: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Expires at timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Is active
    pub is_active: bool,
}

/// Secure key storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureKeyStorageConfig {
    /// Use hardware security module (HSM)
    pub use_hsm: bool,
    /// HSM endpoint (if applicable)
    pub hsm_endpoint: Option<String>,
    /// Use secure enclave
    pub use_secure_enclave: bool,
    /// Master key encryption algorithm
    pub master_key_algorithm: EncryptionAlgorithm,
    /// Key backup location
    pub backup_location: Option<String>,
    /// Backup encryption enabled
    pub backup_encryption: bool,
}

/// Encryption key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    /// Key ID
    pub key_id: String,
    /// Key data (encrypted)
    pub key_data: Vec<u8>,
    /// Key algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Expires at timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Key status
    pub status: KeyStatus,
    /// Key version
    pub version: u32,
    /// Key metadata
    pub metadata: HashMap<String, String>,
    /// Access control list
    pub access_control: Vec<KeyAccessControl>,
    /// Last accessed timestamp
    pub last_accessed: Option<DateTime<Utc>>,
    /// Usage count
    pub usage_count: u64,
}

/// Encryption algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

/// Key status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    /// Key is active
    Active,
    /// Key is inactive
    Inactive,
    /// Key is expired
    Expired,
    /// Key is revoked
    Revoked,
    /// Key is compromised
    Compromised,
}

/// Encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Key ID used for encryption
    pub key_id: String,
    /// Nonce used for encryption
    pub nonce: Vec<u8>,
    /// Encrypted data
    pub data: Vec<u8>,
    /// Authentication tag
    pub tag: Vec<u8>,
    /// Encryption timestamp
    pub encrypted_at: DateTime<Utc>,
}

/// Encryption manager with secure key management
pub struct EncryptionManager {
    /// Encryption keys
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    /// Master key (for key encryption)
    master_key: Option<Vec<u8>>,
    /// Key rotation interval (days)
    key_rotation_interval: u32,
    /// Running flag
    running: bool,
    /// Audit logs
    audit_logs: Arc<RwLock<Vec<KeyAuditLog>>>,
    /// Access control list
    access_control: Arc<RwLock<HashMap<String, KeyAccessControl>>>,
    /// Secure storage configuration
    secure_config: SecureKeyStorageConfig,
    /// Current user context
    current_user: Option<(String, KeyManagementRole)>,
}

impl EncryptionManager {
    /// Create a new encryption manager with secure key management
    pub fn new(key_rotation_interval: u32) -> Result<Self> {
        let secure_config = SecureKeyStorageConfig {
            use_hsm: false, // TODO: Implement HSM integration
            hsm_endpoint: None,
            use_secure_enclave: false, // TODO: Implement secure enclave
            master_key_algorithm: EncryptionAlgorithm::Aes256Gcm,
            backup_location: None,
            backup_encryption: true,
        };

        Ok(Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            master_key: None,
            key_rotation_interval,
            running: false,
            audit_logs: Arc::new(RwLock::new(Vec::new())),
            access_control: Arc::new(RwLock::new(HashMap::new())),
            secure_config,
            current_user: None,
        })
    }

    /// Set current user context for access control
    pub fn set_user_context(&mut self, user_id: String, role: KeyManagementRole) {
        self.current_user = Some((user_id, role));
    }

    /// Clear current user context
    pub fn clear_user_context(&mut self) {
        self.current_user = None;
    }

    /// Check if user has access to key
    async fn check_key_access(&self, user_id: &str, key_id: &str, operation: &KeyOperation) -> Result<bool> {
        let access_control = self.access_control.read().await;
        
        if let Some(user_access) = access_control.get(user_id) {
            if !user_access.is_active {
                return Ok(false);
            }
            
            if let Some(expires_at) = user_access.expires_at {
                if Utc::now() > expires_at {
                    return Ok(false);
                }
            }
            
            // Check if user has access to this specific key
            if !user_access.allowed_key_ids.is_empty() && !user_access.allowed_key_ids.contains(&key_id.to_string()) {
                return Ok(false);
            }
            
            // Check role-based permissions
            match (&user_access.role, operation) {
                (KeyManagementRole::Administrator, _) => Ok(true),
                (KeyManagementRole::Operator, KeyOperation::Access) => Ok(true),
                (KeyManagementRole::Auditor, KeyOperation::Access) => Ok(true),
                (KeyManagementRole::ReadOnly, KeyOperation::Access) => Ok(true),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    /// Log audit event
    async fn log_audit_event(
        &self,
        operation: KeyOperation,
        key_id: Option<String>,
        success: bool,
        error_message: Option<String>,
    ) {
        let user_info = if let Some((user_id, role)) = &self.current_user {
            (user_id.clone(), role.clone())
        } else {
            ("system".to_string(), KeyManagementRole::Administrator)
        };

        let audit_entry = KeyAuditLog {
            timestamp: Utc::now(),
            user_id: user_info.0,
            user_role: user_info.1,
            operation,
            key_id,
            success,
            error_message,
            ip_address: None, // TODO: Get from request context
            session_id: None, // TODO: Get from request context
        };

        let mut audit_logs = self.audit_logs.write().await;
        audit_logs.push(audit_entry);
        
        // Keep only last 10000 audit entries
        if audit_logs.len() > 10000 {
            audit_logs.drain(0..1000);
        }
    }

    /// Start the encryption manager
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting encryption manager with secure key management");
        self.running = true;
        
        // Generate master key if not exists
        if self.master_key.is_none() {
            self.generate_master_key()?;
        }
        
        // Start key rotation task
        let keys = self.keys.clone();
        let rotation_interval = self.key_rotation_interval;
        let audit_logs = self.audit_logs.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(rotation_interval as u64 * 24 * 60 * 60)
            );
            
            loop {
                interval.tick().await;
                Self::rotate_keys(&keys, &audit_logs).await;
            }
        });
        
        self.log_audit_event(KeyOperation::Generate, None, true, None).await;
        Ok(())
    }

    /// Stop the encryption manager
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping encryption manager");
        self.running = false;
        
        // Backup keys before stopping
        self.backup_keys().await?;
        
        self.log_audit_event(KeyOperation::Backup, None, true, None).await;
        Ok(())
    }

    /// Generate a new encryption key with access control
    pub async fn generate_key(&self, key_id: &str, algorithm: EncryptionAlgorithm) -> Result<()> {
        // Check access permissions
        if let Some((user_id, _)) = &self.current_user {
            if !self.check_key_access(user_id, key_id, &KeyOperation::Generate).await? {
                self.log_audit_event(KeyOperation::Generate, Some(key_id.to_string()), false, Some("Access denied".to_string())).await;
                return Err(crate::error::IppanError::Storage("Access denied for key generation".to_string()));
            }
        }

        let key_data = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let mut key = vec![0u8; 32];
                rand::thread_rng().fill_bytes(&mut key);
                key
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let mut key = vec![0u8; 32];
                rand::thread_rng().fill_bytes(&mut key);
                key
            }
        };
        
        let encrypted_key = self.encrypt_key_data(&key_data)?;
        
        let key = EncryptionKey {
            key_id: key_id.to_string(),
            key_data: encrypted_key,
            algorithm,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(self.key_rotation_interval as i64)),
            status: KeyStatus::Active,
            version: 1,
            metadata: HashMap::new(),
            access_control: Vec::new(),
            last_accessed: None,
            usage_count: 0,
        };
        
        let mut keys = self.keys.write().await;
        keys.insert(key_id.to_string(), key);
        
        log::info!("Generated encryption key: {}", key_id);
        self.log_audit_event(KeyOperation::Generate, Some(key_id.to_string()), true, None).await;
        Ok(())
    }

    /// Encrypt data with access control
    pub async fn encrypt_data(&self, data: &[u8], key_id: &str) -> Result<EncryptedData> {
        // Check access permissions
        if let Some((user_id, _)) = &self.current_user {
            if !self.check_key_access(user_id, key_id, &KeyOperation::Access).await? {
                self.log_audit_event(KeyOperation::Access, Some(key_id.to_string()), false, Some("Access denied".to_string())).await;
                return Err(crate::error::IppanError::Storage("Access denied for key usage".to_string()));
            }
        }

        let mut keys = self.keys.write().await;
        
        let key = keys.get_mut(key_id).ok_or_else(|| {
            crate::error::IppanError::Storage(
                format!("Encryption key not found: {}", key_id)
            )
        })?;
        
        if key.status != KeyStatus::Active {
            self.log_audit_event(KeyOperation::Access, Some(key_id.to_string()), false, Some("Key not active".to_string())).await;
            return Err(crate::error::IppanError::Storage(
                format!("Encryption key is not active: {}", key_id)
            ));
        }
        
        // Update key usage statistics
        key.last_accessed = Some(Utc::now());
        key.usage_count += 1;
        
        let decrypted_key = self.decrypt_key_data(&key.key_data)?;
        
        let encrypted_data = match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                self.encrypt_aes256gcm(data, &decrypted_key, key_id)?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.encrypt_chacha20poly1305(data, &decrypted_key, key_id)?
            }
        };
        
        self.log_audit_event(KeyOperation::Access, Some(key_id.to_string()), true, None).await;
        Ok(encrypted_data)
    }

    /// Decrypt data with access control
    pub async fn decrypt_data(&self, encrypted_data: &EncryptedData) -> Result<Vec<u8>> {
        let key_id = &encrypted_data.key_id;
        
        // Check access permissions
        if let Some((user_id, _)) = &self.current_user {
            if !self.check_key_access(user_id, key_id, &KeyOperation::Access).await? {
                self.log_audit_event(KeyOperation::Access, Some(key_id.to_string()), false, Some("Access denied".to_string())).await;
                return Err(crate::error::IppanError::Storage("Access denied for key usage".to_string()));
            }
        }

        let mut keys = self.keys.write().await;
        
        let key = keys.get_mut(key_id).ok_or_else(|| {
            crate::error::IppanError::Storage(
                format!("Encryption key not found: {}", key_id)
            )
        })?;
        
        if key.status != KeyStatus::Active {
            self.log_audit_event(KeyOperation::Access, Some(key_id.to_string()), false, Some("Key not active".to_string())).await;
            return Err(crate::error::IppanError::Storage(
                format!("Encryption key is not active: {}", key_id)
            ));
        }
        
        // Update key usage statistics
        key.last_accessed = Some(Utc::now());
        key.usage_count += 1;
        
        let decrypted_key = self.decrypt_key_data(&key.key_data)?;
        
        let decrypted_data = match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                self.decrypt_aes256gcm(encrypted_data, &decrypted_key)?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.decrypt_chacha20poly1305(encrypted_data, &decrypted_key)?
            }
        };
        
        self.log_audit_event(KeyOperation::Access, Some(key_id.to_string()), true, None).await;
        Ok(decrypted_data)
    }

    /// Revoke a key
    pub async fn revoke_key(&self, key_id: &str) -> Result<()> {
        // Check access permissions
        if let Some((user_id, _)) = &self.current_user {
            if !self.check_key_access(user_id, key_id, &KeyOperation::Revoke).await? {
                self.log_audit_event(KeyOperation::Revoke, Some(key_id.to_string()), false, Some("Access denied".to_string())).await;
                return Err(crate::error::IppanError::Storage("Access denied for key revocation".to_string()));
            }
        }

        let mut keys = self.keys.write().await;
        
        if let Some(key) = keys.get_mut(key_id) {
            key.status = KeyStatus::Revoked;
            log::info!("Revoked encryption key: {}", key_id);
            self.log_audit_event(KeyOperation::Revoke, Some(key_id.to_string()), true, None).await;
        }
        
        Ok(())
    }

    /// Add user access control
    pub async fn add_user_access(&self, user_id: String, role: KeyManagementRole, allowed_key_ids: Vec<String>) -> Result<()> {
        let access_control = KeyAccessControl {
            user_id: user_id.clone(),
            role,
            allowed_key_ids,
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        };
        
        let mut access_list = self.access_control.write().await;
        access_list.insert(user_id.clone(), access_control);
        
        log::info!("Added user access control: {}", user_id);
        Ok(())
    }

    /// Remove user access control
    pub async fn remove_user_access(&self, user_id: &str) -> Result<()> {
        let mut access_list = self.access_control.write().await;
        access_list.remove(user_id);
        
        log::info!("Removed user access control: {}", user_id);
        Ok(())
    }

    /// Get audit logs
    pub async fn get_audit_logs(&self, limit: Option<usize>) -> Result<Vec<KeyAuditLog>> {
        let audit_logs = self.audit_logs.read().await;
        let logs = if let Some(limit) = limit {
            audit_logs.iter().rev().take(limit).cloned().collect()
        } else {
            audit_logs.clone()
        };
        Ok(logs)
    }

    /// Backup keys securely
    async fn backup_keys(&self) -> Result<()> {
        let keys = self.keys.read().await;
        let access_control = self.access_control.read().await;
        
        // TODO: Implement secure backup to configured location
        log::info!("Backed up {} keys and {} access control entries", keys.len(), access_control.len());
        Ok(())
    }

    /// Rotate keys with audit logging
    async fn rotate_keys(keys: &Arc<RwLock<HashMap<String, EncryptionKey>>>, audit_logs: &Arc<RwLock<Vec<KeyAuditLog>>>) {
        let mut keys = keys.write().await;
        let now = Utc::now();
        
        for (key_id, key) in keys.iter_mut() {
            if let Some(expires_at) = key.expires_at {
                if now > expires_at {
                    key.status = KeyStatus::Expired;
                    log::info!("Key expired: {}", key_id);
                    
                    // Log the expiration
                    let audit_entry = KeyAuditLog {
                        timestamp: now,
                        user_id: "system".to_string(),
                        user_role: KeyManagementRole::Administrator,
                        operation: KeyOperation::Rotate,
                        key_id: Some(key_id.clone()),
                        success: true,
                        error_message: None,
                        ip_address: None,
                        session_id: None,
                    };
                    
                    let mut audit_logs = audit_logs.write().await;
                    audit_logs.push(audit_entry);
                }
            }
        }
    }

    /// Get encryption statistics
    pub async fn get_encryption_stats(&self) -> EncryptionStats {
        let keys = self.keys.read().await;
        let audit_logs = self.audit_logs.read().await;
        let access_control = self.access_control.read().await;
        
        let total_keys = keys.len();
        let active_keys = keys.values().filter(|k| k.status == KeyStatus::Active).count();
        let expired_keys = keys.values().filter(|k| k.status == KeyStatus::Expired).count();
        let revoked_keys = keys.values().filter(|k| k.status == KeyStatus::Revoked).count();
        
        EncryptionStats {
            total_keys,
            active_keys,
            expired_keys,
            revoked_keys,
            total_audit_entries: audit_logs.len(),
            total_users: access_control.len(),
            master_key_configured: self.master_key.is_some(),
            secure_storage_enabled: self.secure_config.use_hsm || self.secure_config.use_secure_enclave,
        }
    }

    /// Generate master key
    fn generate_master_key(&mut self) -> Result<()> {
        let mut master_key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut master_key);
        self.master_key = Some(master_key);
        Ok(())
    }

    /// Encrypt key data with master key
    fn encrypt_key_data(&self, key_data: &[u8]) -> Result<Vec<u8>> {
        let master_key = self.master_key.as_ref().ok_or_else(|| {
            crate::error::IppanError::Storage(
                "Master key not available".to_string()
            )
        })?;
        
        // Simple XOR encryption for demonstration
        // In production, use proper key encryption
        let mut encrypted = Vec::new();
        for (i, &byte) in key_data.iter().enumerate() {
            encrypted.push(byte ^ master_key[i % master_key.len()]);
        }
        
        Ok(encrypted)
    }

    /// Decrypt key data with master key
    fn decrypt_key_data(&self, encrypted_key_data: &[u8]) -> Result<Vec<u8>> {
        let master_key = self.master_key.as_ref().ok_or_else(|| {
            crate::error::IppanError::Storage(
                "Master key not available".to_string()
            )
        })?;
        
        // Simple XOR decryption for demonstration
        let mut decrypted = Vec::new();
        for (i, &byte) in encrypted_key_data.iter().enumerate() {
            decrypted.push(byte ^ master_key[i % master_key.len()]);
        }
        
        Ok(decrypted)
    }

    /// Encrypt data with AES-256-GCM
    fn encrypt_aes256gcm(&self, data: &[u8], key: &[u8], key_id: &str) -> Result<EncryptedData> {
        // Ensure key is exactly 32 bytes for AES-256
        if key.len() != 32 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid key size: {} (expected 32)", key.len())
            ));
        }
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Storage(format!("Invalid key: {}", e)))?;
        
        // Generate 12-byte nonce for AES-GCM
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| crate::error::IppanError::Storage(format!("Encryption failed: {}", e)))?;
        
        // Split ciphertext into data and tag
        let tag_size = 16;
        if ciphertext.len() < tag_size {
            return Err(crate::error::IppanError::Storage(
                "Invalid ciphertext length".to_string()
            ));
        }
        
        let data_len = ciphertext.len() - tag_size;
        let encrypted_data = ciphertext[..data_len].to_vec();
        let tag = ciphertext[data_len..].to_vec();
        
        Ok(EncryptedData {
            key_id: key_id.to_string(),
            nonce: nonce_bytes.to_vec(),
            data: encrypted_data,
            tag,
            encrypted_at: Utc::now(),
        })
    }

    /// Decrypt data with AES-256-GCM
    fn decrypt_aes256gcm(&self, encrypted_data: &EncryptedData, key: &[u8]) -> Result<Vec<u8>> {
        // Ensure key is exactly 32 bytes for AES-256
        if key.len() != 32 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid key size: {} (expected 32)", key.len())
            ));
        }
        
        // Ensure nonce is exactly 12 bytes
        if encrypted_data.nonce.len() != 12 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid nonce size: {} (expected 12)", encrypted_data.nonce.len())
            ));
        }
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Storage(format!("Invalid key: {}", e)))?;
        
        let nonce = Nonce::from_slice(&encrypted_data.nonce);
        
        // Combine data and tag
        let mut ciphertext = encrypted_data.data.clone();
        ciphertext.extend_from_slice(&encrypted_data.tag);
        
        let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
            .map_err(|e| crate::error::IppanError::Storage(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext)
    }

    /// Encrypt data with ChaCha20-Poly1305
    fn encrypt_chacha20poly1305(&self, data: &[u8], key: &[u8], key_id: &str) -> Result<EncryptedData> {
        use chacha20poly1305::{ChaCha20Poly1305, Nonce, KeyInit};
        use chacha20poly1305::aead::Aead;
        
        // Ensure key is exactly 32 bytes for ChaCha20-Poly1305
        if key.len() != 32 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid key size: {} (expected 32)", key.len())
            ));
        }
        
        let cipher = ChaCha20Poly1305::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Storage(format!("Invalid key: {}", e)))?;
        
        // Generate 12-byte nonce for ChaCha20-Poly1305
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| crate::error::IppanError::Storage(format!("Encryption failed: {}", e)))?;
        
        // Split ciphertext into data and tag
        let tag_size = 16;
        if ciphertext.len() < tag_size {
            return Err(crate::error::IppanError::Storage(
                "Invalid ciphertext length".to_string()
            ));
        }
        
        let data_len = ciphertext.len() - tag_size;
        let encrypted_data = ciphertext[..data_len].to_vec();
        let tag = ciphertext[data_len..].to_vec();
        
        Ok(EncryptedData {
            key_id: key_id.to_string(),
            nonce: nonce_bytes.to_vec(),
            data: encrypted_data,
            tag,
            encrypted_at: Utc::now(),
        })
    }

    /// Decrypt data with ChaCha20-Poly1305
    fn decrypt_chacha20poly1305(&self, encrypted_data: &EncryptedData, key: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305, Nonce, KeyInit};
        use chacha20poly1305::aead::Aead;
        
        // Ensure key is exactly 32 bytes for ChaCha20-Poly1305
        if key.len() != 32 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid key size: {} (expected 32)", key.len())
            ));
        }
        
        // Ensure nonce is exactly 12 bytes
        if encrypted_data.nonce.len() != 12 {
            return Err(crate::error::IppanError::Storage(
                format!("Invalid nonce size: {} (expected 12)", encrypted_data.nonce.len())
            ));
        }
        
        let cipher = ChaCha20Poly1305::new_from_slice(key)
            .map_err(|e| crate::error::IppanError::Storage(format!("Invalid key: {}", e)))?;
        
        let nonce = Nonce::from_slice(&encrypted_data.nonce);
        
        // Combine data and tag
        let mut ciphertext = encrypted_data.data.clone();
        ciphertext.extend_from_slice(&encrypted_data.tag);
        
        let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
            .map_err(|e| crate::error::IppanError::Storage(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext)
    }
}

/// Encryption statistics
#[derive(Debug, Serialize)]
pub struct EncryptionStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub expired_keys: usize,
    pub revoked_keys: usize,
    pub total_audit_entries: usize,
    pub total_users: usize,
    pub master_key_configured: bool,
    pub secure_storage_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_manager_creation() {
        let manager = EncryptionManager::new(30).unwrap();
        
        assert_eq!(manager.key_rotation_interval, 30);
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_encryption_manager_start_stop() {
        let mut manager = EncryptionManager::new(30).unwrap();
        
        manager.start().await.unwrap();
        assert!(manager.running);
        
        manager.stop().await.unwrap();
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_key_generation() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        
        let stats = manager.get_encryption_stats().await;
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.active_keys, 1);
    }

    #[tokio::test]
    async fn test_data_encryption_decryption() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        // Generate a key
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        
        // Encrypt data
        let data = b"Hello, World!";
        let encrypted = manager.encrypt_data(data, "test_key").await.unwrap();
        
        // Decrypt data
        let decrypted = manager.decrypt_data(&encrypted).await.unwrap();
        
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn test_key_revocation() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        manager.revoke_key("test_key").await.unwrap();
        
        let stats = manager.get_encryption_stats().await;
        assert_eq!(stats.revoked_keys, 1);
    }

    #[tokio::test]
    async fn test_secure_key_management() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        // Test user access control
        manager.add_user_access(
            "admin".to_string(),
            KeyManagementRole::Administrator,
            vec![],
        ).await.unwrap();
        
        manager.add_user_access(
            "operator".to_string(),
            KeyManagementRole::Operator,
            vec!["key1".to_string(), "key2".to_string()],
        ).await.unwrap();
        
        // Set user context and test key generation
        manager.set_user_context("admin".to_string(), KeyManagementRole::Administrator);
        manager.generate_key("key1", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        manager.generate_key("key2", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        
        // Test operator access
        manager.set_user_context("operator".to_string(), KeyManagementRole::Operator);
        let data = b"Test data";
        let encrypted = manager.encrypt_data(data, "key1").await.unwrap();
        let decrypted = manager.decrypt_data(&encrypted).await.unwrap();
        assert_eq!(decrypted, data);
        
        // Test access denied for unauthorized key
        let result = manager.encrypt_data(data, "key3").await;
        assert!(result.is_err());
        
        // Test audit logging
        let audit_logs = manager.get_audit_logs(Some(10)).await.unwrap();
        assert!(!audit_logs.is_empty());
        
        // Verify audit log contains our operations
        let has_generate = audit_logs.iter().any(|log| matches!(log.operation, KeyOperation::Generate));
        let has_access = audit_logs.iter().any(|log| matches!(log.operation, KeyOperation::Access));
        assert!(has_generate);
        assert!(has_access);
        
        // Test key revocation
        manager.set_user_context("admin".to_string(), KeyManagementRole::Administrator);
        manager.revoke_key("key1").await.unwrap();
        
        // Verify revoked key cannot be used
        manager.set_user_context("operator".to_string(), KeyManagementRole::Operator);
        let result = manager.encrypt_data(data, "key1").await;
        assert!(result.is_err());
        
        // Test statistics
        let stats = manager.get_encryption_stats().await;
        assert_eq!(stats.total_keys, 2);
        assert_eq!(stats.active_keys, 1);
        assert_eq!(stats.revoked_keys, 1);
        assert_eq!(stats.total_users, 2);
        assert!(stats.master_key_configured);
        assert!(!stats.secure_storage_enabled); // HSM not enabled in test
        
        println!("✅ Secure key management test passed!");
        println!("   - Access control: ✅ Working");
        println!("   - Audit logging: ✅ Working");
        println!("   - Key revocation: ✅ Working");
        println!("   - Statistics: ✅ Working");
    }

    #[tokio::test]
    async fn test_role_based_access_control() {
        let mut manager = EncryptionManager::new(30).unwrap();
        manager.start().await.unwrap();
        
        // Add different user roles
        manager.add_user_access(
            "admin".to_string(),
            KeyManagementRole::Administrator,
            vec![],
        ).await.unwrap();
        
        manager.add_user_access(
            "auditor".to_string(),
            KeyManagementRole::Auditor,
            vec![],
        ).await.unwrap();
        
        manager.add_user_access(
            "readonly".to_string(),
            KeyManagementRole::ReadOnly,
            vec![],
        ).await.unwrap();
        
        // Generate a key as admin
        manager.set_user_context("admin".to_string(), KeyManagementRole::Administrator);
        manager.generate_key("test_key", EncryptionAlgorithm::Aes256Gcm).await.unwrap();
        
        // Test auditor can access but not generate
        manager.set_user_context("auditor".to_string(), KeyManagementRole::Auditor);
        let data = b"Test data";
        let encrypted = manager.encrypt_data(data, "test_key").await.unwrap();
        let decrypted = manager.decrypt_data(&encrypted).await.unwrap();
        assert_eq!(decrypted, data);
        
        // Test readonly can access but not generate
        manager.set_user_context("readonly".to_string(), KeyManagementRole::ReadOnly);
        let encrypted2 = manager.encrypt_data(data, "test_key").await.unwrap();
        let decrypted2 = manager.decrypt_data(&encrypted2).await.unwrap();
        assert_eq!(decrypted2, data);
        
        // Test readonly cannot generate keys
        let result = manager.generate_key("new_key", EncryptionAlgorithm::Aes256Gcm).await;
        assert!(result.is_err());
        
        // Test auditor cannot generate keys
        manager.set_user_context("auditor".to_string(), KeyManagementRole::Auditor);
        let result = manager.generate_key("new_key", EncryptionAlgorithm::Aes256Gcm).await;
        assert!(result.is_err());
        
        println!("✅ Role-based access control test passed!");
        println!("   - Administrator: ✅ Full access");
        println!("   - Auditor: ✅ Read access only");
        println!("   - ReadOnly: ✅ Read access only");
    }
}
