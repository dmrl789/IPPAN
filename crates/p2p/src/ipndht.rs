use crate::Libp2pNetwork;
use anyhow::{Context, Result};
use async_trait::async_trait;
use blake3;
use ippan_files::descriptor::FileDescriptor;
use ippan_files::dht::{DhtLookupResult, DhtPublishResult, FileDhtService};
use ippan_files::FileId;
use ippan_l2_handle_registry::{
    dht::{HandleDhtError, HandleDhtRecord, HandleDhtService},
    Handle,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// Minimal metadata cached for files that have been published either locally or
/// discovered through the DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedDescriptor {
    descriptor: FileDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedHandleRecord {
    record: HandleDhtRecord,
}

/// IPNDHT helper responsible for publishing and discovering file descriptors.
#[derive(Clone)]
pub struct IpnDhtService {
    network: Option<Arc<Libp2pNetwork>>,
    cache: Arc<RwLock<HashMap<FileId, CachedDescriptor>>>,
    handle_cache: Arc<RwLock<HashMap<Handle, CachedHandleRecord>>>,
}

impl IpnDhtService {
    /// Create a new DHT service. If `network` is `None`, operations fall back to the local cache.
    pub fn new(network: Option<Arc<Libp2pNetwork>>) -> Self {
        Self {
            network,
            cache: Arc::new(RwLock::new(HashMap::new())),
            handle_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Publish a descriptor to the DHT (if configured) and cache it locally.
    pub async fn publish_file(&self, descriptor: &FileDescriptor) -> Result<()> {
        self.cache.write().insert(
            descriptor.id,
            CachedDescriptor {
                descriptor: descriptor.clone(),
            },
        );

        if let Some(network) = &self.network {
            let key = Self::key_for(&descriptor.id);
            let value = serde_json::to_vec(descriptor).context("serialize descriptor for DHT")?;
            network
                .put_dht_record(key.clone(), value)
                .context("publish descriptor to DHT")?;
            network
                .start_providing_record(key)
                .context("announce descriptor provider")?;
        }

        Ok(())
    }

    /// Attempt to locate a descriptor locally or via the DHT.
    pub async fn find_file(&self, id: &FileId) -> Result<Option<FileDescriptor>> {
        if let Some(record) = self.cache.read().get(id) {
            return Ok(Some(record.descriptor.clone()));
        }

        let Some(network) = &self.network else {
            return Ok(None);
        };

        if let Some(bytes) = network
            .get_dht_record(Self::key_for(id))
            .await
            .context("query descriptor record from DHT")?
        {
            let descriptor: FileDescriptor =
                serde_json::from_slice(&bytes).context("decode descriptor from DHT")?;
            if let Some(descriptor) = self.cache_descriptor_from_dht(id, descriptor) {
                return Ok(Some(descriptor));
            }
            return Ok(None);
        }

        Ok(None)
    }

    /// Query libp2p for provider peer IDs advertising the descriptor key.
    pub async fn provider_peers(&self, id: &FileId) -> Result<Vec<String>> {
        let Some(network) = &self.network else {
            return Ok(Vec::new());
        };

        let peers = network
            .get_dht_providers(Self::key_for(id))
            .await
            .context("query descriptor providers from DHT")?;
        Ok(peers.into_iter().map(|peer| peer.to_string()).collect())
    }

    fn key_for(id: &FileId) -> Vec<u8> {
        id.as_bytes().to_vec()
    }

    fn cache_descriptor_from_dht(
        &self,
        requested_id: &FileId,
        descriptor: FileDescriptor,
    ) -> Option<FileDescriptor> {
        if descriptor.id != *requested_id {
            warn!(
                ?requested_id,
                received = ?descriptor.id,
                "Rejected descriptor with mismatched id"
            );
            return None;
        }

        let mut cache = self.cache.write();
        if let Some(existing) = cache.get(requested_id) {
            if existing.descriptor != descriptor {
                warn!(?requested_id, "Rejected conflicting descriptor fields");
                return Some(existing.descriptor.clone());
            }
        }

        cache.insert(
            descriptor.id,
            CachedDescriptor {
                descriptor: descriptor.clone(),
            },
        );
        Some(descriptor)
    }

    fn cache_handle_from_dht(
        &self,
        requested_handle: &Handle,
        record: HandleDhtRecord,
    ) -> Option<HandleDhtRecord> {
        if record.handle != *requested_handle {
            warn!(
                ?requested_handle,
                received = %record.handle.as_str(),
                "Rejected handle record with mismatched handle"
            );
            return None;
        }

        let mut cache = self.handle_cache.write();
        if let Some(existing) = cache.get(requested_handle) {
            if existing.record != record {
                warn!(
                    handle = %requested_handle.as_str(),
                    "Rejected conflicting handle record fields"
                );
                return Some(existing.record.clone());
            }
        }

        cache.insert(
            record.handle.clone(),
            CachedHandleRecord {
                record: record.clone(),
            },
        );
        Some(record)
    }

    pub async fn publish_handle(&self, record: &HandleDhtRecord) -> Result<()> {
        self.handle_cache.write().insert(
            record.handle.clone(),
            CachedHandleRecord {
                record: record.clone(),
            },
        );

        if let Some(network) = &self.network {
            let key = Self::handle_key(&record.handle);
            let value = serde_json::to_vec(record).context("serialize handle record for DHT")?;
            network
                .put_dht_record(key.clone(), value)
                .context("publish handle record to DHT")?;
            network
                .start_providing_record(key)
                .context("announce handle record provider")?;
        }

        Ok(())
    }

    pub async fn find_handle(&self, handle: &Handle) -> Result<Option<HandleDhtRecord>> {
        if let Some(record) = self.handle_cache.read().get(handle) {
            return Ok(Some(record.record.clone()));
        }

        let Some(network) = &self.network else {
            return Ok(None);
        };

        if let Some(bytes) = network
            .get_dht_record(Self::handle_key(handle))
            .await
            .context("query handle record from DHT")?
        {
            let record: HandleDhtRecord =
                serde_json::from_slice(&bytes).context("decode handle record from DHT")?;
            if let Some(record) = self.cache_handle_from_dht(handle, record) {
                return Ok(Some(record));
            }
            return Ok(None);
        }

        Ok(None)
    }

    fn handle_key(handle: &Handle) -> Vec<u8> {
        let digest = blake3::hash(handle.as_str().as_bytes());
        let mut key = b"handle:".to_vec();
        key.extend_from_slice(digest.as_bytes());
        key
    }
}

/// File DHT service backed by a shared `IpnDhtService` handle.
#[derive(Clone)]
pub struct Libp2pFileDhtService {
    inner: Arc<IpnDhtService>,
}

impl Libp2pFileDhtService {
    pub fn new(service: Arc<IpnDhtService>) -> Self {
        Self { inner: service }
    }
}

#[async_trait]
impl FileDhtService for Libp2pFileDhtService {
    async fn publish_file(&self, descriptor: &FileDescriptor) -> Result<DhtPublishResult> {
        self.inner.publish_file(descriptor).await?;
        Ok(DhtPublishResult {
            file_id: descriptor.id,
            success: true,
            message: Some("libp2p: descriptor published".to_string()),
        })
    }

    async fn find_file(&self, id: &FileId) -> Result<DhtLookupResult> {
        let descriptor = self.inner.find_file(id).await?;
        let providers = self.inner.provider_peers(id).await?;
        Ok(DhtLookupResult {
            file_id: *id,
            descriptor,
            providers,
        })
    }
}

/// Handle DHT service backed by a shared `IpnDhtService` handle.
#[derive(Clone)]
pub struct Libp2pHandleDhtService {
    inner: Arc<IpnDhtService>,
}

impl Libp2pHandleDhtService {
    pub fn new(service: Arc<IpnDhtService>) -> Self {
        Self { inner: service }
    }
}

#[async_trait]
impl HandleDhtService for Libp2pHandleDhtService {
    async fn publish_handle(&self, record: &HandleDhtRecord) -> Result<(), HandleDhtError> {
        self.inner
            .publish_handle(record)
            .await
            .map_err(HandleDhtError::from)
    }

    async fn find_handle(
        &self,
        handle: &Handle,
    ) -> Result<Option<HandleDhtRecord>, HandleDhtError> {
        self.inner
            .find_handle(handle)
            .await
            .map_err(HandleDhtError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_files::descriptor::{ContentHash, FileDescriptor};
    use ippan_l2_handle_registry::PublicKey;

    fn sample_descriptor() -> FileDescriptor {
        let owner = [1u8; 32];
        let hash = ContentHash::from_data(b"sample");
        FileDescriptor::new(
            hash,
            owner,
            1024,
            Some("application/octet-stream".into()),
            vec!["test".into()],
        )
    }

    #[tokio::test]
    async fn publishes_and_finds_from_cache() {
        let service = IpnDhtService::new(None);
        let descriptor = sample_descriptor();
        service.publish_file(&descriptor).await.expect("publish");

        let fetched = service
            .find_file(&descriptor.id)
            .await
            .expect("find")
            .expect("descriptor");
        assert_eq!(fetched.id, descriptor.id);
        assert_eq!(fetched.owner, descriptor.owner);
    }

    #[tokio::test]
    async fn libp2p_file_dht_service_wraps_ipndht() {
        let ipn = Arc::new(IpnDhtService::new(None));
        let service = Libp2pFileDhtService::new(ipn);
        let descriptor = sample_descriptor();

        let publish = service
            .publish_file(&descriptor)
            .await
            .expect("publish result");
        assert!(publish.success);

        let lookup = service
            .find_file(&descriptor.id)
            .await
            .expect("lookup result");
        assert_eq!(lookup.file_id, descriptor.id);
        assert_eq!(lookup.providers.len(), 0);
    }

    #[test]
    fn rejects_mismatched_descriptor_from_dht() {
        let service = IpnDhtService::new(None);
        let descriptor = sample_descriptor();
        let other = sample_descriptor();
        assert_ne!(descriptor.id, other.id);

        let result = service.cache_descriptor_from_dht(&descriptor.id, other.clone());
        assert!(result.is_none());
        assert!(service.cache.read().get(&other.id).is_none());
        assert!(service.cache.read().get(&descriptor.id).is_none());
    }

    #[test]
    fn conflicting_descriptor_keeps_cached_version() {
        let service = IpnDhtService::new(None);
        let descriptor = sample_descriptor();
        service.cache.write().insert(
            descriptor.id,
            CachedDescriptor {
                descriptor: descriptor.clone(),
            },
        );

        let mut conflicting = descriptor.clone();
        conflicting.size_bytes += 512;

        let returned = service
            .cache_descriptor_from_dht(&descriptor.id, conflicting)
            .expect("cached descriptor returned");

        assert_eq!(returned.id, descriptor.id);
        assert_eq!(returned.size_bytes, descriptor.size_bytes);

        let cache_guard = service.cache.read();
        let cached = cache_guard.get(&descriptor.id).unwrap();
        assert_eq!(cached.descriptor.size_bytes, descriptor.size_bytes);
    }

    fn sample_handle_record() -> HandleDhtRecord {
        HandleDhtRecord::new(Handle::new("@demo.ipn"), PublicKey([9u8; 32]), Some(42))
    }

    #[test]
    fn rejects_mismatched_handle_from_dht() {
        let service = IpnDhtService::new(None);
        let expected = sample_handle_record();
        let other = HandleDhtRecord::new(
            Handle::new("@other.ipn"),
            expected.owner.clone(),
            expected.expires_at,
        );

        let result = service.cache_handle_from_dht(&expected.handle, other);
        assert!(result.is_none());
        assert!(service.handle_cache.read().is_empty());
    }

    #[test]
    fn conflicting_handle_record_keeps_cached_version() {
        let service = IpnDhtService::new(None);
        let record = sample_handle_record();
        service.handle_cache.write().insert(
            record.handle.clone(),
            CachedHandleRecord {
                record: record.clone(),
            },
        );

        let mut conflicting = record.clone();
        conflicting.expires_at = Some(999);

        let returned = service
            .cache_handle_from_dht(&record.handle, conflicting)
            .expect("cached handle returned");

        assert_eq!(returned.owner, record.owner);
        assert_eq!(returned.expires_at, record.expires_at);
    }

    #[tokio::test]
    async fn publishes_and_finds_handle_from_cache() {
        let service = IpnDhtService::new(None);
        let record = sample_handle_record();
        service.publish_handle(&record).await.expect("publish");

        let fetched = service
            .find_handle(&record.handle)
            .await
            .expect("lookup")
            .expect("record");
        assert_eq!(fetched.handle.as_str(), record.handle.as_str());
        assert_eq!(fetched.owner.as_bytes(), record.owner.as_bytes());
    }

    #[tokio::test]
    async fn libp2p_handle_dht_service_wraps_ipndht() {
        let ipn = Arc::new(IpnDhtService::new(None));
        let service = Libp2pHandleDhtService::new(ipn);
        let record = sample_handle_record();

        service
            .publish_handle(&record)
            .await
            .expect("publish handle");

        let fetched = service
            .find_handle(&record.handle)
            .await
            .expect("lookup handle")
            .expect("record present");
        assert_eq!(fetched.expires_at, record.expires_at);
    }
}
