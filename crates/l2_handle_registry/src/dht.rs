use crate::{Handle, PublicKey};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur while interacting with the handle DHT layer.
#[derive(Debug, Error)]
pub enum HandleDhtError {
    #[error("handle DHT backend error: {0}")]
    Backend(String),
}

impl From<anyhow::Error> for HandleDhtError {
    fn from(value: anyhow::Error) -> Self {
        Self::Backend(value.to_string())
    }
}

/// Record published into the DHT for a given handle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandleDhtRecord {
    pub handle: Handle,
    pub owner: PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

impl HandleDhtRecord {
    pub fn new(handle: Handle, owner: PublicKey, expires_at: Option<u64>) -> Self {
        Self {
            handle,
            owner,
            expires_at,
        }
    }
}

#[async_trait]
pub trait HandleDhtService: Send + Sync {
    async fn publish_handle(&self, record: &HandleDhtRecord) -> Result<(), HandleDhtError>;

    async fn find_handle(&self, handle: &Handle)
        -> Result<Option<HandleDhtRecord>, HandleDhtError>;
}

/// Stub implementation backed by an in-memory hashmap.
#[derive(Clone, Default)]
pub struct StubHandleDhtService {
    records: Arc<RwLock<HashMap<Handle, HandleDhtRecord>>>,
}

impl StubHandleDhtService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, handle: &Handle) -> Option<HandleDhtRecord> {
        self.records.read().get(handle).cloned()
    }
}

#[async_trait]
impl HandleDhtService for StubHandleDhtService {
    async fn publish_handle(&self, record: &HandleDhtRecord) -> Result<(), HandleDhtError> {
        self.records
            .write()
            .insert(record.handle.clone(), record.clone());
        Ok(())
    }

    async fn find_handle(
        &self,
        handle: &Handle,
    ) -> Result<Option<HandleDhtRecord>, HandleDhtError> {
        Ok(self.records.read().get(handle).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_publish_and_find() {
        let stub = StubHandleDhtService::new();
        let handle = Handle::new("@demo.ipn");
        let record = HandleDhtRecord::new(handle.clone(), PublicKey([5u8; 32]), Some(123));

        stub.publish_handle(&record)
            .await
            .expect("publish succeeds");

        let fetched = stub
            .find_handle(&handle)
            .await
            .expect("lookup succeeds")
            .expect("record present");
        assert_eq!(fetched.owner, record.owner);
        assert_eq!(fetched.expires_at, record.expires_at);
    }
}
