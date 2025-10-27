//! L2 Handle Registry implementation

use crate::errors::*;
use crate::types::*;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

/// L2 Handle Registry
///
/// Stores human-readable handle mappings and metadata on L2.
/// L1 only stores ownership anchors pointing to this registry.
#[derive(Debug)]
pub struct L2HandleRegistry {
    /// Handle to metadata mapping
    handles: Arc<RwLock<HashMap<Handle, HandleMetadata>>>,
    /// Public key to handles mapping (for reverse lookup)
    owner_to_handles: Arc<RwLock<HashMap<PublicKey, Vec<Handle>>>>,
}

impl L2HandleRegistry {
    /// Create a new L2 handle registry
    pub fn new() -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
            owner_to_handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new handle
    pub fn register(&self, registration: HandleRegistration) -> Result<()> {
        // Validate handle format
        if !registration.handle.is_valid() {
            return Err(HandleRegistryError::InvalidHandleFormat {
                handle: registration.handle.as_str().to_string(),
            });
        }

        // Check if handle already exists
        {
            let handles = self.handles.read();
            if handles.contains_key(&registration.handle) {
                return Err(HandleRegistryError::HandleAlreadyExists {
                    handle: registration.handle.as_str().to_string(),
                });
            }
        }

        // Verify signature (simplified - in production, use proper crypto)
        if !self.verify_signature(&registration.owner, &registration.signature) {
            return Err(HandleRegistryError::Unauthorized {
                handle: registration.handle.as_str().to_string(),
            });
        }

        // Create handle metadata
        let mut metadata = HandleMetadata {
            owner: registration.owner.clone(),
            expires_at: registration.expires_at.unwrap_or(0),
            metadata: registration.metadata,
            ..Default::default()
        };

        // Set L1 anchor (would be provided by L1 in production)
        metadata.l1_anchor =
            Some(self.compute_l1_anchor(&registration.handle, &registration.owner));

        // Store handle
        {
            let mut handles = self.handles.write();
            handles.insert(registration.handle.clone(), metadata);
        }

        // Update owner mapping
        {
            let mut owner_map = self.owner_to_handles.write();
            owner_map
                .entry(registration.owner)
                .or_insert_with(Vec::new)
                .push(registration.handle);
        }

        Ok(())
    }

    /// Update handle metadata
    pub fn update(&self, update: HandleUpdate) -> Result<()> {
        // Verify ownership and signature
        {
            let handles = self.handles.read();
            if let Some(metadata) = handles.get(&update.handle) {
                if metadata.owner != update.owner {
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

        if !self.verify_signature(&update.owner, &update.signature) {
            return Err(HandleRegistryError::Unauthorized {
                handle: update.handle.as_str().to_string(),
            });
        }

        // Update metadata
        {
            let mut handles = self.handles.write();
            if let Some(metadata) = handles.get_mut(&update.handle) {
                metadata.updated_at = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                metadata.metadata.extend(update.updates);
            }
        }

        Ok(())
    }

    /// Transfer handle ownership
    pub fn transfer(&self, transfer: HandleTransfer) -> Result<()> {
        // Verify current ownership
        {
            let handles = self.handles.read();
            if let Some(metadata) = handles.get(&transfer.handle) {
                if metadata.owner != transfer.from_owner {
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

        if !self.verify_signature(&transfer.from_owner, &transfer.signature) {
            return Err(HandleRegistryError::Unauthorized {
                handle: transfer.handle.as_str().to_string(),
            });
        }

        // Update ownership
        {
            let mut handles = self.handles.write();
            if let Some(metadata) = handles.get_mut(&transfer.handle) {
                metadata.owner = transfer.to_owner.clone();
                metadata.updated_at = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }

        // Update owner mappings
        {
            let mut owner_map = self.owner_to_handles.write();

            // Remove from old owner
            if let Some(handles) = owner_map.get_mut(&transfer.from_owner) {
                handles.retain(|h| h != &transfer.handle);
            }

            // Add to new owner
            owner_map
                .entry(transfer.to_owner)
                .or_insert_with(Vec::new)
                .push(transfer.handle);
        }

        Ok(())
    }

    /// Resolve handle to public key
    pub fn resolve(&self, handle: &Handle) -> Result<PublicKey> {
        let handles = self.handles.read();
        if let Some(metadata) = handles.get(handle) {
            // Check if handle is expired
            if metadata.expires_at > 0
                && metadata.expires_at
                    < SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
            {
                return Err(HandleRegistryError::HandleExpired {
                    handle: handle.as_str().to_string(),
                });
            }

            Ok(metadata.owner.clone())
        } else {
            Err(HandleRegistryError::HandleNotFound {
                handle: handle.as_str().to_string(),
            })
        }
    }

    /// Get handle metadata
    pub fn get_metadata(&self, handle: &Handle) -> Result<HandleMetadata> {
        let handles = self.handles.read();
        handles
            .get(handle)
            .cloned()
            .ok_or_else(|| HandleRegistryError::HandleNotFound {
                handle: handle.as_str().to_string(),
            })
    }

    /// List handles owned by a public key
    pub fn list_owner_handles(&self, owner: &PublicKey) -> Vec<Handle> {
        let owner_map = self.owner_to_handles.read();
        owner_map.get(owner).cloned().unwrap_or_default()
    }

    /// Compute L1 anchor hash
    fn compute_l1_anchor(&self, handle: &Handle, owner: &PublicKey) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(handle.as_str().as_bytes());
        hasher.update(owner.as_bytes());
        hasher.finalize().into()
    }

    /// Verify signature (simplified - in production, use proper Ed25519 verification)
    fn verify_signature(&self, _owner: &PublicKey, _signature: &[u8]) -> bool {
        // In production, this would verify Ed25519 signatures
        // For now, just return true for testing
        true
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
    fn test_handle_registration() {
        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@test.ipn");
        let owner = PublicKey::new([1u8; 32]);

        let registration = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: vec![1, 2, 3],
            metadata: HashMap::new(),
            expires_at: None,
        };

        assert!(registry.register(registration).is_ok());
        assert_eq!(registry.resolve(&handle).unwrap(), owner);
    }

    #[test]
    fn test_handle_not_found() {
        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@nonexistent.ipn");

        assert!(registry.resolve(&handle).is_err());
    }

    #[test]
    fn test_handle_validation() {
        assert!(Handle::new("@valid.ipn").is_valid());
        assert!(Handle::new("@device.iot").is_valid());
        assert!(Handle::new("@premium.cyborg").is_valid());
        assert!(!Handle::new("invalid").is_valid());
        assert!(!Handle::new("@").is_valid());
    }
}
