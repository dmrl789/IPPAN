use crate::Libp2pNetwork;
use anyhow::{Context, Result};
use ippan_types::{FileDescriptor, FileDescriptorId};
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
    cache: Arc<RwLock<HashMap<FileDescriptorId, CachedDescriptor>>>,
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
            let key = descriptor.id.to_bytes().to_vec();
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
    pub async fn find_file(&self, id: &FileDescriptorId) -> Result<Option<FileDescriptor>> {
        if let Some(record) = self.cache.read().get(id) {
            return Ok(Some(record.descriptor.clone()));
        }

        let Some(network) = &self.network else {
            return Ok(None);
        };

        if let Some(bytes) = network
            .get_dht_record(id.to_bytes().to_vec())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{random_nonce, Address, HashTimer, IppanTimeMicros};

    fn sample_descriptor() -> FileDescriptor {
        let owner = Address([1u8; 32]);
        let time = IppanTimeMicros(42);
        let nonce = random_nonce();
        let node_id = [9u8; 32];
        let hashtimer = HashTimer::derive("file", time, b"files", &[0u8; 16], &nonce, &node_id);
        FileDescriptor::new(
            hashtimer,
            owner,
            [2u8; 32],
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
}
