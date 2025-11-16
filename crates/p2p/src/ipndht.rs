use crate::Libp2pNetwork;
use anyhow::{Context, Result};
use async_trait::async_trait;
use ippan_files::descriptor::FileDescriptor;
use ippan_files::dht::{DhtLookupResult, DhtPublishResult, FileDhtService};
use ippan_files::FileId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Minimal metadata cached for files that have been published either locally or
/// discovered through the DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedDescriptor {
    descriptor: FileDescriptor,
}

/// IPNDHT helper responsible for publishing and discovering file descriptors.
#[derive(Clone)]
pub struct IpnDhtService {
    network: Option<Arc<Libp2pNetwork>>,
    cache: Arc<RwLock<HashMap<FileId, CachedDescriptor>>>,
}

impl IpnDhtService {
    /// Create a new DHT service. If `network` is `None`, operations fall back to the local cache.
    pub fn new(network: Option<Arc<Libp2pNetwork>>) -> Self {
        Self {
            network,
            cache: Arc::new(RwLock::new(HashMap::new())),
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
            self.cache.write().insert(
                descriptor.id,
                CachedDescriptor {
                    descriptor: descriptor.clone(),
                },
            );
            return Ok(Some(descriptor));
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

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_files::descriptor::{ContentHash, FileDescriptor};

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
}
