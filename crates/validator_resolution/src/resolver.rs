//! Validator ID resolver implementation
//!
//! Resolves human-readable handles or public keys to deterministic validator identities.
//!
//! Sources used in resolution:
//! 1. Direct Ed25519 public key (no lookup needed)
//! 2. L2 Handle Registry (@handle.ipn)
//! 3. L1 Ownership Anchor (chain-anchored ownership records)
//! 4. Registry alias (reserved internal identifiers)

use crate::errors::*;
use crate::types::{ResolutionMethod, ResolvedValidator, ValidatorMetadata};
use ippan_economics::ValidatorId;
use ippan_l1_handle_anchors::L1HandleAnchorStorage;
use ippan_l2_handle_registry::{Handle, L2HandleRegistry};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Validator ID resolver
///
/// Provides unified resolution of validator identities across
/// L1 and L2 namespaces. Used by consensus and governance modules.
#[derive(Debug)]
pub struct ValidatorResolver {
    l2_registry: Arc<L2HandleRegistry>,
    l1_anchors: Arc<L1HandleAnchorStorage>,
    cache: Arc<parking_lot::RwLock<std::collections::HashMap<ValidatorId, CachedEntry>>>,
    cache_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedEntry {
    value: ResolvedValidator,
    inserted_at: Instant,
}

impl ValidatorResolver {
    /// Create a new validator resolver
    pub fn new(l2_registry: Arc<L2HandleRegistry>, l1_anchors: Arc<L1HandleAnchorStorage>) -> Self {
        Self {
            l2_registry,
            l1_anchors,
            cache: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
            cache_ttl: Duration::from_secs(300),
        }
    }

    /// Resolve a single `ValidatorId` into a public key and metadata
    pub async fn resolve(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        if let Some(cached_entry) = self.get_from_cache(id) {
            if self.is_cache_valid(&cached_entry) {
                return Ok(cached_entry.value.clone());
            }
        }

        let resolved = match self.resolve_method(id) {
            ResolutionMethod::Direct => self.resolve_direct(id).await?,
            ResolutionMethod::L2HandleRegistry => self.resolve_via_l2_handle(id).await?,
            ResolutionMethod::L1OwnershipAnchor => self.resolve_via_l1_anchor(id).await?,
            ResolutionMethod::RegistryAlias => self.resolve_via_alias(id).await?,
        };

        self.store_in_cache(id, &resolved);
        Ok(resolved)
    }

    /// Resolve multiple ValidatorIds in parallel
    pub async fn resolve_batch(
        &self,
        ids: &[ValidatorId],
    ) -> std::collections::HashMap<ValidatorId, Result<ResolvedValidator>> {
        let mut results = std::collections::HashMap::new();
        let mut futures = Vec::new();

        for id in ids {
            let resolver = self.clone();
            let id_clone = id.clone();
            let fut = async move {
                let result = resolver.resolve(&id_clone).await;
                (id_clone, result)
            };
            futures.push(fut);
        }

        let batch_results = futures::future::join_all(futures).await;
        for (id, result) in batch_results {
            results.insert(id, result);
        }

        results
    }

    /// Determine resolution method for a given ID
    fn resolve_method(&self, id: &ValidatorId) -> ResolutionMethod {
        if id.is_public_key() {
            ResolutionMethod::Direct
        } else if id.is_handle() {
            ResolutionMethod::L2HandleRegistry
        } else {
            ResolutionMethod::RegistryAlias
        }
    }

    /// Direct resolution for Ed25519 public keys
    async fn resolve_direct(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let public_key = self.parse_public_key(id.as_str())?;
        Ok(ResolvedValidator::new(
            id.clone(),
            public_key,
            ResolutionMethod::Direct,
        ))
    }

    /// Resolve handle via L2 Handle Registry
    async fn resolve_via_l2_handle(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let handle = Handle::new(id.as_str());
        let registry = self.l2_registry.clone();
        let handle_clone = handle.clone();

        let l2_public_key = tokio::task::spawn_blocking(move || registry.resolve(&handle_clone))
            .await
            .map_err(|_| ValidatorResolutionError::ResolutionTimeout)??;

        let public_key = *l2_public_key.as_bytes();
        let metadata = self.get_handle_metadata(&handle).await.ok();

        Ok(ResolvedValidator::with_metadata(
            id.clone(),
            public_key,
            ResolutionMethod::L2HandleRegistry,
            metadata.unwrap_or_else(|| ValidatorMetadata {
                handle: Some(id.as_str().to_string()),
                created_at: None,
                updated_at: None,
                status: None,
                custom: std::collections::HashMap::new(),
            }),
        ))
    }

    /// Resolve validator ownership via L1 anchors
    async fn resolve_via_l1_anchor(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let anchor = self.l1_anchors.get_anchor_by_handle(id.as_str())?;
        Ok(ResolvedValidator::new(
            id.clone(),
            anchor.owner,
            ResolutionMethod::L1OwnershipAnchor,
        ))
    }

    /// Registry alias fallback (reserved internal identifiers)
    async fn resolve_via_alias(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        Err(ValidatorResolutionError::InvalidFormat {
            id: id.as_str().to_string(),
        })
    }

    /// Retrieve handle metadata from the L2 registry
    async fn get_handle_metadata(&self, handle: &Handle) -> Result<ValidatorMetadata> {
        let registry = self.l2_registry.clone();
        let handle_clone = handle.clone();

        let metadata = tokio::task::spawn_blocking(move || registry.get_metadata(&handle_clone))
            .await
            .map_err(|_| ValidatorResolutionError::ResolutionTimeout)??;

        Ok(ValidatorMetadata {
            handle: Some(handle.as_str().to_string()),
            created_at: Some(metadata.created_at),
            updated_at: Some(metadata.updated_at),
            status: Some(format!("{:?}", metadata.status)),
            custom: metadata.metadata,
        })
    }

    /// Parse public key from hex string
    fn parse_public_key(&self, hex_str: &str) -> Result<[u8; 32]> {
        if hex_str.len() != 64 {
            return Err(ValidatorResolutionError::InvalidPublicKey);
        }
        let mut key = [0u8; 32];
        hex::decode_to_slice(hex_str, &mut key)
            .map_err(|_| ValidatorResolutionError::InvalidPublicKey)?;
        Ok(key)
    }

    // -------------------------------------------------------------------------
    // Cache Management
    // -------------------------------------------------------------------------
    fn get_from_cache(&self, id: &ValidatorId) -> Option<CachedEntry> {
        self.cache.read().get(id).cloned()
    }

    fn store_in_cache(&self, id: &ValidatorId, resolved: &ResolvedValidator) {
        let entry = CachedEntry {
            value: resolved.clone(),
            inserted_at: Instant::now(),
        };
        self.cache.write().insert(id.clone(), entry);
    }

    fn is_cache_valid(&self, entry: &CachedEntry) -> bool {
        entry.inserted_at.elapsed() < self.cache_ttl
    }

    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    #[cfg(test)]
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }
}

impl Clone for ValidatorResolver {
    fn clone(&self) -> Self {
        Self {
            l2_registry: self.l2_registry.clone(),
            l1_anchors: self.l1_anchors.clone(),
            cache: self.cache.clone(),
            cache_ttl: self.cache_ttl,
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use ippan_l1_handle_anchors::HandleOwnershipAnchor;
    use ippan_l2_handle_registry::{Handle, HandleRegistration, HandleUpdate, PublicKey};
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::time::Duration;

    #[tokio::test]
    async fn test_direct_resolution() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry, l1_anchors);

        let id =
            ValidatorId::new("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let resolved = resolver.resolve(&id).await.unwrap();

        assert_eq!(resolved.resolution_method, ResolutionMethod::Direct);
        assert_eq!(resolved.public_key_bytes()[0], 0x01);
    }

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signed_registration(handle: &str, signing_key: &SigningKey) -> HandleRegistration {
        let handle = Handle::new(handle);
        let owner = PublicKey::new(signing_key.verifying_key().to_bytes());

        let mut message = Vec::new();
        message.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        message.extend_from_slice(handle.as_str().as_bytes());
        message.extend_from_slice(owner.as_bytes());
        let message_hash = Sha256::digest(&message);
        let signature = signing_key.sign(&message_hash);

        HandleRegistration {
            handle,
            owner,
            signature: signature.to_bytes().to_vec(),
            metadata: HashMap::new(),
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn test_handle_resolution() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry.clone(), l1_anchors);

        let handle = "@test.ipn";
        let registration = signed_registration(handle, &signing_key(42));
        let expected_owner = registration.owner.clone();
        l2_registry.register(registration).unwrap();

        let id = ValidatorId::new(handle);
        let resolved = resolver.resolve(&id).await.unwrap();

        assert_eq!(
            resolved.resolution_method,
            ResolutionMethod::L2HandleRegistry
        );
        assert_eq!(resolved.public_key_bytes(), expected_owner.as_bytes());
        let metadata = resolved.metadata.expect("metadata present");
        assert_eq!(metadata.handle.as_deref(), Some(handle));
        assert_eq!(metadata.status.as_deref(), Some("Active"));
    }

    #[tokio::test]
    async fn test_resolve_alias_error_and_cache() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let mut resolver = ValidatorResolver::new(l2_registry, l1_anchors);

        resolver.set_cache_ttl(Duration::from_secs(10));

        let alias = ValidatorId::new("validator-alias");
        let err = resolver.resolve(&alias).await.unwrap_err();
        assert!(matches!(
            err,
            ValidatorResolutionError::InvalidFormat { id } if id == "validator-alias"
        ));

        let pk_id =
            ValidatorId::new("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        let first = resolver.resolve(&pk_id).await.unwrap();
        let second = resolver.resolve(&pk_id).await.unwrap();
        assert_eq!(first.public_key, second.public_key);
    }

    #[tokio::test]
    async fn test_l1_anchor_resolution_path() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry, l1_anchors.clone());

        let handle = "@anchor.ipn";
        let owner = [7u8; 32];
        let anchor = HandleOwnershipAnchor::new(handle, owner, [9u8; 32], 10, 5, vec![1]);
        l1_anchors.store_anchor(anchor.clone()).unwrap();

        let id = ValidatorId::new(handle);
        let resolved = resolver
            .resolve_via_l1_anchor(&id)
            .await
            .expect("resolved via l1");
        assert_eq!(resolved.public_key_bytes(), &owner);
        assert_eq!(
            resolved.resolution_method,
            ResolutionMethod::L1OwnershipAnchor
        );
    }

    #[tokio::test]
    async fn test_resolve_batch_mixed_results() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry.clone(), l1_anchors);

        // Register one handle with metadata update to exercise metadata path.
        let handle = "@batch.ipn";
        let signing_key = signing_key(5);
        let registration = signed_registration(handle, &signing_key);
        let owner = registration.owner.clone();
        l2_registry.register(registration).unwrap();

        let mut update_metadata = HashMap::new();
        update_metadata.insert("role".to_string(), "validator".to_string());
        let mut update_message = Vec::new();
        update_message.extend_from_slice(b"IPPAN_HANDLE_UPDATE");
        update_message.extend_from_slice(handle.as_bytes());
        update_message.extend_from_slice(owner.as_bytes());
        let update_hash = Sha256::digest(&update_message);
        let update_signature = signing_key.sign(&update_hash);
        l2_registry
            .update(HandleUpdate {
                handle: Handle::new(handle),
                owner: owner.clone(),
                signature: update_signature.to_bytes().to_vec(),
                updates: update_metadata,
            })
            .unwrap();

        let ids = vec![
            ValidatorId::new(handle),
            ValidatorId::new("zz"),
            ValidatorId::new("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcd"),
        ];

        let results = resolver.resolve_batch(&ids).await;
        assert_eq!(results.len(), 3);
        let handle_result = results.get(&ValidatorId::new(handle)).unwrap();
        let resolved = handle_result.as_ref().unwrap();
        assert_eq!(resolved.public_key_bytes(), owner.as_bytes());
        assert!(resolved
            .metadata
            .as_ref()
            .unwrap()
            .custom
            .contains_key("role"));

        let alias_result = results
            .get(&ValidatorId::new("zz"))
            .unwrap()
            .as_ref()
            .unwrap_err();
        assert!(matches!(
            alias_result,
            ValidatorResolutionError::InvalidFormat { .. }
        ));
    }

    #[tokio::test]
    async fn test_parse_public_key_validation() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry, l1_anchors);

        assert!(resolver
            .parse_public_key("abcd")
            .unwrap_err()
            .to_string()
            .contains("Invalid public key"));
        assert!(resolver
            .parse_public_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcg")
            .is_err());
    }
}
