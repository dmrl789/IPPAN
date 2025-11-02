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

        if !self.verify_signature(&registration.owner, &registration.signature) {
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
                .or_insert_with(Vec::new)
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

        if !self.verify_signature(&update.owner, &update.signature) {
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

        if !self.verify_signature(&transfer.from_owner, &transfer.signature) {
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
                .or_insert_with(Vec::new)
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

    /// Dummy signature verification placeholder
    fn verify_signature(&self, _owner: &PublicKey, _sig: &[u8]) -> bool {
        true
    }
}

impl Default for L2HandleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_handle_registration_and_resolution() {
        let registry = L2HandleRegistry::new();
        let handle = Handle::new("@test.ipn");
        let owner = PublicKey::new([1u8; 32]);

        let reg = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: vec![1, 2, 3],
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
}
