//! L2 Handle Registry implementation
//!
//! Provides Layer-2 human-readable handle mapping
//! and metadata management (e.g. `@alice.ipn`, `@device.iot`).

use crate::errors::*;
use crate::types::*;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

/// L2 Handle Registry
///
/// Stores human-readable handle mappings and metadata on L2.
/// L1 only stores ownership anchors pointing to this registry.
#[derive(Debug)]
pub struct L2HandleRegistry {
    /// Handle → metadata mapping
    handles: Arc<RwLock<HashMap<Handle, HandleMetadata>>>,
    /// Owner public key → list of handles
    owner_to_handles: Arc<RwLock<HashMap<PublicKey, Vec<Handle>>>>,
}

impl L2HandleRegistry {
    /// Create a new handle registry
    pub fn new() -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
            owner_to_handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new handle
    pub fn register(&self, registration: HandleRegistration) -> Result<()> {
        if !registration.handle.is_valid() {
            return Err(HandleRegistryError::InvalidHandleFormat {
                handle: registration.handle.as_str().to_string(),
            });
        }

        {
            let handles = self.handles.read();
            if handles.contains_key(&registration.handle) {
                return Err(HandleRegistryError::HandleAlreadyExists {
                    handle: registration.handle.as_str().to_string(),
                });
            }
        }

        if !self.verify_registration_signature(&registration) {
            return Err(HandleRegistryError::Unauthorized {
                handle: registration.handle.as_str().to_string(),
            });
        }

        let mut metadata = HandleMetadata {
            owner: registration.owner.clone(),
            expires_at: registration.expires_at.unwrap_or(0),
            metadata: registration.metadata,
            ..Default::default()
        };

        metadata.l1_anchor =
            Some(self.compute_l1_anchor(&registration.handle, &registration.owner));

        {
            let mut handles = self.handles.write();
            handles.insert(registration.handle.clone(), metadata);
        }

        {
            let mut map = self.owner_to_handles.write();
            map.entry(registration.owner)
                .or_default()
                .push(registration.handle);
        }

        Ok(())
    }

    /// Update handle metadata
    pub fn update(&self, update: HandleUpdate) -> Result<()> {
        {
            let handles = self.handles.read();
            if let Some(meta) = handles.get(&update.handle) {
                if meta.owner != update.owner {
                    return Err(HandleRegistryError::Unauthorized {
                        handle: update.handle.as_str().to_string(),
                    });
                }
            } else {
                return Err(HandleRegistryError::HandleNotFound {
                    handle: update.handle.as_str().to_string(),
                });
            }
        }

        if !self.verify_update_signature(&update) {
            return Err(HandleRegistryError::Unauthorized {
                handle: update.handle.as_str().to_string(),
            });
        }

        {
            let mut handles = self.handles.write();
            if let Some(meta) = handles.get_mut(&update.handle) {
                meta.updated_at = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                meta.metadata.extend(update.updates);
            }
        }

        Ok(())
    }

    /// Transfer handle ownership
    pub fn transfer(&self, transfer: HandleTransfer) -> Result<()> {
        {
            let handles = self.handles.read();
            if let Some(meta) = handles.get(&transfer.handle) {
                if meta.owner != transfer.from_owner {
                    return Err(HandleRegistryError::Unauthorized {
                        handle: transfer.handle.as_str().to_string(),
                    });
                }
            } else {
                return Err(HandleRegistryError::HandleNotFound {
                    handle: transfer.handle.as_str().to_string(),
                });
            }
        }

        if !self.verify_transfer_signature(&transfer) {
            return Err(HandleRegistryError::Unauthorized {
                handle: transfer.handle.as_str().to_string(),
            });
        }

        {
            let mut handles = self.handles.write();
            if let Some(meta) = handles.get_mut(&transfer.handle) {
                meta.owner = transfer.to_owner.clone();
                meta.updated_at = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }

        {
            let mut map = self.owner_to_handles.write();
            if let Some(list) = map.get_mut(&transfer.from_owner) {
                list.retain(|h| h != &transfer.handle);
            }
            map.entry(transfer.to_owner)
                .or_default()
                .push(transfer.handle);
        }

        Ok(())
    }

    /// Resolve handle → owner key
    pub fn resolve(&self, handle: &Handle) -> Result<PublicKey> {
        let handles = self.handles.read();
        if let Some(meta) = handles.get(handle) {
            if meta.expires_at > 0
                && meta.expires_at
                    < SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
            {
                return Err(HandleRegistryError::HandleExpired {
                    handle: handle.as_str().to_string(),
                });
            }
            Ok(meta.owner.clone())
        } else {
            Err(HandleRegistryError::HandleNotFound {
                handle: handle.as_str().to_string(),
            })
        }
    }

    /// Fetch handle metadata
    pub fn get_metadata(&self, handle: &Handle) -> Result<HandleMetadata> {
        let handles = self.handles.read();
        handles
            .get(handle)
            .cloned()
            .ok_or_else(|| HandleRegistryError::HandleNotFound {
                handle: handle.as_str().to_string(),
            })
    }

    /// List all handles owned by a public key
    pub fn list_owner_handles(&self, owner: &PublicKey) -> Vec<Handle> {
        let map = self.owner_to_handles.read();
        map.get(owner).cloned().unwrap_or_default()
    }

    /// Compute deterministic L1 anchor hash
    fn compute_l1_anchor(&self, handle: &Handle, owner: &PublicKey) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(handle.as_str().as_bytes());
        h.update(owner.as_bytes());
        h.finalize().into()
    }

    /// Verify signature for handle registration
    fn verify_registration_signature(&self, registration: &HandleRegistration) -> bool {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        use sha2::{Digest, Sha256};

        if registration.signature.len() != 64 {
            return false;
        }

        // Parse public key
        let Ok(verifying_key) = VerifyingKey::from_bytes(registration.owner.as_bytes()) else {
            return false;
        };

        // Parse signature
        let Ok(signature) = Signature::from_slice(&registration.signature) else {
            return false;
        };

        // Construct the message that should have been signed
        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        message.extend_from_slice(registration.handle.as_str().as_bytes());
        message.extend_from_slice(registration.owner.as_bytes());
        if let Some(expires) = registration.expires_at {
            message.extend_from_slice(&expires.to_le_bytes());
        }

        // Hash the message
        let message_hash = Sha256::digest(&message);

        // Verify signature
        verifying_key.verify(&message_hash, &signature).is_ok()
    }

    /// Verify signature for handle update
    fn verify_update_signature(&self, update: &HandleUpdate) -> bool {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        use sha2::{Digest, Sha256};

        if update.signature.len() != 64 {
            return false;
        }

        let Ok(verifying_key) = VerifyingKey::from_bytes(update.owner.as_bytes()) else {
            return false;
        };

        let Ok(signature) = Signature::from_slice(&update.signature) else {
            return false;
        };

        // Construct the message
        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_UPDATE");
        message.extend_from_slice(update.handle.as_str().as_bytes());
        message.extend_from_slice(update.owner.as_bytes());

        let message_hash = Sha256::digest(&message);
        verifying_key.verify(&message_hash, &signature).is_ok()
    }

    /// Verify signature for handle transfer
    fn verify_transfer_signature(&self, transfer: &HandleTransfer) -> bool {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        use sha2::{Digest, Sha256};

        if transfer.signature.len() != 64 {
            return false;
        }

        let Ok(verifying_key) = VerifyingKey::from_bytes(transfer.from_owner.as_bytes()) else {
            return false;
        };

        let Ok(signature) = Signature::from_slice(&transfer.signature) else {
            return false;
        };

        // Construct the message
        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_TRANSFER");
        message.extend_from_slice(transfer.handle.as_str().as_bytes());
        message.extend_from_slice(transfer.from_owner.as_bytes());
        message.extend_from_slice(transfer.to_owner.as_bytes());

        let message_hash = Sha256::digest(&message);
        verifying_key.verify(&message_hash, &signature).is_ok()
    }
}

impl Default for L2HandleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_registration_and_resolution() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@test.ipn");

        // Generate a proper key pair
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let owner = PublicKey::new(signing_key.verifying_key().to_bytes());

        // Create proper signature
        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        message.extend_from_slice(handle.as_str().as_bytes());
        message.extend_from_slice(owner.as_bytes());
        let message_hash = Sha256::digest(&message);
        let signature = signing_key.sign(&message_hash);

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: signature.to_bytes().to_vec(),
            metadata: HashMap::new(),
            expires_at: None,
        };

        assert!(registry.register(reg).is_ok());
        assert_eq!(registry.resolve(&handle).unwrap(), owner);
    }

    #[test]
    fn test_handle_not_found() {
        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@missing.ipn");
        assert!(registry.resolve(&handle).is_err());
    }

    #[test]
    fn test_handle_format_validation() {
        assert!(Handle::new("@valid.ipn").is_valid());
        assert!(Handle::new("@device.iot").is_valid());
        assert!(Handle::new("@premium.cyborg").is_valid());
        assert!(!Handle::new("invalid").is_valid());
        assert!(!Handle::new("@").is_valid());
    }

    #[test]
    fn test_signature_verification_fails_with_wrong_key() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@test.ipn");

        // Sign with one key
        let signing_key1 = SigningKey::from_bytes(&[42u8; 32]);
        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        message.extend_from_slice(handle.as_str().as_bytes());
        message.extend_from_slice(&signing_key1.verifying_key().to_bytes());
        let message_hash = Sha256::digest(&message);
        let signature = signing_key1.sign(&message_hash);

        // But claim a different owner
        let signing_key2 = SigningKey::from_bytes(&[99u8; 32]);
        let wrong_owner = PublicKey::new(signing_key2.verifying_key().to_bytes());

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: wrong_owner,
            signature: signature.to_bytes().to_vec(),
            metadata: HashMap::new(),
            expires_at: None,
        };

        // Should fail due to signature mismatch
        assert!(registry.register(reg).is_err());
    }

    #[test]
    fn test_signature_verification_fails_with_invalid_signature() {
        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@test.ipn");
        let owner = PublicKey::new([1u8; 32]);

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: vec![1, 2, 3], // Invalid signature
            metadata: HashMap::new(),
            expires_at: None,
        };

        // Should fail due to invalid signature format
        assert!(registry.register(reg).is_err());
    }

    #[test]
    fn test_handle_transfer_with_proper_signature() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@test.ipn");

        // Register handle with first owner
        let signing_key1 = SigningKey::from_bytes(&[42u8; 32]);
        let owner1 = PublicKey::new(signing_key1.verifying_key().to_bytes());

        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        message.extend_from_slice(handle.as_str().as_bytes());
        message.extend_from_slice(owner1.as_bytes());
        let message_hash = Sha256::digest(&message);
        let signature = signing_key1.sign(&message_hash);

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: owner1.clone(),
            signature: signature.to_bytes().to_vec(),
            metadata: HashMap::new(),
            expires_at: None,
        };

        registry.register(reg).unwrap();

        // Transfer to second owner
        let signing_key2 = SigningKey::from_bytes(&[99u8; 32]);
        let owner2 = PublicKey::new(signing_key2.verifying_key().to_bytes());

        let mut transfer_message = Vec::new();
        transfer_message.extend_from_slice(b"IPPAN_HANDLE_TRANSFER");
        transfer_message.extend_from_slice(handle.as_str().as_bytes());
        transfer_message.extend_from_slice(owner1.as_bytes());
        transfer_message.extend_from_slice(owner2.as_bytes());
        let transfer_hash = Sha256::digest(&transfer_message);
        let transfer_sig = signing_key1.sign(&transfer_hash);

        let transfer = HandleTransfer {
            handle: handle.clone(),
            from_owner: owner1.clone(),
            to_owner: owner2.clone(),
            signature: transfer_sig.to_bytes().to_vec(),
        };

        assert!(registry.transfer(transfer).is_ok());
        assert_eq!(registry.resolve(&handle).unwrap(), owner2);
    }

    #[test]
    fn test_handle_update_merges_metadata() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@update.ipn");

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let owner = PublicKey::new(signing_key.verifying_key().to_bytes());

        // Register handle
        let mut registration_message = Vec::new();
        registration_message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        registration_message.extend_from_slice(handle.as_str().as_bytes());
        registration_message.extend_from_slice(owner.as_bytes());
        let registration_hash = Sha256::digest(&registration_message);
        let registration_sig = signing_key.sign(&registration_hash);

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: registration_sig.to_bytes().to_vec(),
            metadata: HashMap::new(),
            expires_at: None,
        };
        registry.register(reg).unwrap();

        // Prepare update signature
        let mut update_message = Vec::new();
        update_message.extend_from_slice(b"IPPAN_HANDLE_UPDATE");
        update_message.extend_from_slice(handle.as_str().as_bytes());
        update_message.extend_from_slice(owner.as_bytes());
        let update_hash = Sha256::digest(&update_message);
        let update_sig = signing_key.sign(&update_hash);

        let mut updates = HashMap::new();
        updates.insert("alias".into(), "alice".into());

        let update = HandleUpdate {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: update_sig.to_bytes().to_vec(),
            updates,
        };

        registry.update(update).unwrap();

        let metadata = registry.get_metadata(&handle).unwrap();
        assert_eq!(metadata.metadata.get("alias"), Some(&"alice".to_string()));
        assert!(metadata.updated_at >= metadata.created_at);
    }

    #[test]
    fn test_handle_expiration_blocks_resolution() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@expiring.ipn");
        let signing_key = SigningKey::from_bytes(&[11u8; 32]);
        let owner = PublicKey::new(signing_key.verifying_key().to_bytes());
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(10);

        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        message.extend_from_slice(handle.as_str().as_bytes());
        message.extend_from_slice(owner.as_bytes());
        message.extend_from_slice(&expires_at.to_le_bytes());
        let hash = Sha256::digest(&message);
        let signature = signing_key.sign(&hash);

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: signature.to_bytes().to_vec(),
            metadata: HashMap::new(),
            expires_at: Some(expires_at),
        };

        registry.register(reg).unwrap();
        let err = registry.resolve(&handle).unwrap_err();
        assert!(matches!(err, HandleRegistryError::HandleExpired { .. }));
    }

    #[test]
    fn test_list_owner_handles_returns_all_handles() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        let registry = L2HandleRegistry::new();
        let signing_key = SigningKey::from_bytes(&[55u8; 32]);
        let owner = PublicKey::new(signing_key.verifying_key().to_bytes());

        for suffix in ["one", "two", "three"] {
            let handle = Handle::new(format!("@multi-{}.ipn", suffix));
            let mut message = Vec::new();
            message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
            message.extend_from_slice(handle.as_str().as_bytes());
            message.extend_from_slice(owner.as_bytes());
            let hash = Sha256::digest(&message);
            let signature = signing_key.sign(&hash);

            let reg = HandleRegistration {
                handle: handle.clone(),
                owner: owner.clone(),
                signature: signature.to_bytes().to_vec(),
                metadata: HashMap::new(),
                expires_at: None,
            };
            registry.register(reg).unwrap();
        }

        let handles = registry.list_owner_handles(&owner);
        assert_eq!(handles.len(), 3);
        assert!(handles.iter().any(|h| h.as_str().contains("one")));
    }
}
