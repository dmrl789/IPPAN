//! Handle resolution service for L2 lookups

use crate::errors::*;
use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

/// Handle resolution service
///
/// Provides async resolution of human-readable handles to public keys
/// with caching and fallback mechanisms.
#[derive(Debug)]
pub struct HandleResolver {
    registry: Arc<L2HandleRegistry>,
    cache: Arc<parking_lot::RwLock<HashMap<Handle, (PublicKey, u64)>>>,
    cache_ttl: Duration,
}

impl HandleResolver {
    /// Create a new handle resolver
    pub fn new(registry: Arc<L2HandleRegistry>) -> Self {
        Self {
            registry,
            cache: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Resolve handle to public key with caching
    pub async fn resolve(&self, handle: &Handle) -> Result<PublicKey> {
        // Check cache first
        if let Some((cached_key, timestamp)) = self.get_from_cache(handle) {
            if self.is_cache_valid(timestamp) {
                return Ok(cached_key);
            }
        }

        // Resolve from registry with timeout
        let resolution_result =
            timeout(Duration::from_secs(5), self.registry.resolve(handle)).await;

        match resolution_result {
            Ok(Ok(public_key)) => {
                // Cache the result
                self.store_in_cache(handle, &public_key);
                Ok(public_key)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(HandleRegistryError::ResolutionTimeout),
        }
    }

    /// Resolve multiple handles in parallel
    pub async fn resolve_batch(&self, handles: &[Handle]) -> HashMap<Handle, Result<PublicKey>> {
        let mut results = HashMap::new();
        let mut futures = Vec::new();

        for handle in handles {
            let resolver = self.clone();
            let handle = handle.clone();
            futures.push(async move { (handle, resolver.resolve(&handle).await) });
        }

        let batch_results = futures::future::join_all(futures).await;
        for (handle, result) in batch_results {
            results.insert(handle, result);
        }

        results
    }

    /// Get handle metadata
    pub async fn get_metadata(&self, handle: &Handle) -> Result<HandleMetadata> {
        timeout(Duration::from_secs(5), self.registry.get_metadata(handle))
            .await
            .map_err(|_| HandleRegistryError::ResolutionTimeout)?
    }

    /// List all handles for an owner
    pub async fn list_owner_handles(&self, owner: &PublicKey) -> Vec<Handle> {
        self.registry.list_owner_handles(owner)
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, Duration) {
        let cache = self.cache.read();
        (cache.len(), self.cache_ttl)
    }

    fn get_from_cache(&self, handle: &Handle) -> Option<(PublicKey, u64)> {
        let cache = self.cache.read();
        cache.get(handle).cloned()
    }

    fn store_in_cache(&self, handle: &Handle, public_key: &PublicKey) {
        let timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut cache = self.cache.write();
        cache.insert(handle.clone(), (public_key.clone(), timestamp));
    }

    fn is_cache_valid(&self, timestamp: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now - timestamp < self.cache_ttl.as_secs()
    }
}

impl Clone for HandleResolver {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            cache: self.cache.clone(),
            cache_ttl: self.cache_ttl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_handle_resolution() {
        let registry = Arc::new(L2HandleRegistry::new());
        let resolver = HandleResolver::new(registry.clone());

        // Register a handle
        let handle = Handle::new("@test.ipn");
        let owner = PublicKey::new([1u8; 32]);

        let registration = HandleRegistration {
            handle: handle.clone(),
            owner: owner.clone(),
            signature: vec![1, 2, 3],
            metadata: HashMap::new(),
            expires_at: None,
        };

        registry.register(registration).unwrap();

        // Resolve handle
        let resolved = resolver.resolve(&handle).await.unwrap();
        assert_eq!(resolved, owner);
    }

    #[tokio::test]
    async fn test_batch_resolution() {
        let registry = Arc::new(L2HandleRegistry::new());
        let resolver = HandleResolver::new(registry.clone());

        // Register multiple handles
        let handles = vec![
            Handle::new("@alice.ipn"),
            Handle::new("@bob.ipn"),
            Handle::new("@carol.ipn"),
        ];

        for (i, handle) in handles.iter().enumerate() {
            let owner = PublicKey::new([i as u8; 32]);
            let registration = HandleRegistration {
                handle: handle.clone(),
                owner,
                signature: vec![1, 2, 3],
                metadata: HashMap::new(),
                expires_at: None,
            };
            registry.register(registration).unwrap();
        }

        // Resolve all handles
        let results = resolver.resolve_batch(&handles).await;
        assert_eq!(results.len(), 3);

        for (i, handle) in handles.iter().enumerate() {
            assert!(results.get(handle).unwrap().is_ok());
        }
    }
}
