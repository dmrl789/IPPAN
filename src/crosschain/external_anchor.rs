//! External anchor system for cross-chain and L2 integration
//! 
//! This module handles external chain anchors and L2 anchor events,
//! providing a bridge between different blockchain networks.

use crate::crosschain::types::{AnchorEvent, ProofType, L2CommitTx, L2ExitTx};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// External anchor data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAnchorData {
    /// External chain identifier
    pub chain_id: String,
    /// External state root
    pub state_root: String,
    /// Proof type
    pub proof_type: ProofType,
    /// Proof data
    pub proof_data: Vec<u8>,
    /// Timestamp when anchored
    pub timestamp: u64,
}

/// L2 anchor event handler
pub struct L2AnchorHandler {
    /// Storage for anchor events
    anchor_events: Arc<RwLock<HashMap<String, Vec<AnchorEvent>>>>,
    /// Event subscribers
    subscribers: Arc<RwLock<Vec<Box<dyn AnchorEventSubscriber + Send + Sync>>>>,
}

/// Trait for subscribing to anchor events
pub trait AnchorEventSubscriber {
    /// Called when a new anchor event is created
    fn on_anchor_event(&self, event: &AnchorEvent);
}

impl L2AnchorHandler {
    /// Create a new L2 anchor handler
    pub fn new() -> Self {
        Self {
            anchor_events: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Handle a new L2 commit and create anchor event
    pub async fn handle_l2_commit(
        &self,
        commit: &L2CommitTx,
        timestamp: u64,
    ) -> Result<AnchorEvent, String> {
        // Create anchor event
        let event = AnchorEvent {
            l2_id: commit.l2_id.clone(),
            epoch: commit.epoch,
            state_root: commit.state_root,
            da_hash: commit.da_hash,
            committed_at: timestamp,
        };
        
        // Store the event
        {
            let mut events = self.anchor_events.write().await;
            let l2_events = events.entry(commit.l2_id.clone()).or_insert_with(Vec::new);
            l2_events.push(event.clone());
        }
        
        // Notify subscribers
        self.notify_subscribers(&event).await;
        
        info!("Created L2 anchor event for {} at epoch {}", commit.l2_id, commit.epoch);
        Ok(event)
    }
    
    /// Get anchor events for a specific L2
    pub async fn get_l2_events(&self, l2_id: &str) -> Vec<AnchorEvent> {
        let events = self.anchor_events.read().await;
        events.get(l2_id).cloned().unwrap_or_default()
    }
    
    /// Get latest anchor event for a specific L2
    pub async fn get_latest_l2_event(&self, l2_id: &str) -> Option<AnchorEvent> {
        let events = self.anchor_events.read().await;
        events.get(l2_id)?.last().cloned()
    }
    
    /// Subscribe to anchor events
    pub async fn subscribe(&self, subscriber: Box<dyn AnchorEventSubscriber + Send + Sync>) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(subscriber);
    }
    
    /// Notify all subscribers of a new event
    async fn notify_subscribers(&self, event: &AnchorEvent) {
        let subscribers = self.subscribers.read().await;
        for subscriber in subscribers.iter() {
            subscriber.on_anchor_event(event);
        }
    }
    
    /// Get all anchor events
    pub async fn get_all_events(&self) -> HashMap<String, Vec<AnchorEvent>> {
        let events = self.anchor_events.read().await;
        events.clone()
    }
    
    /// Clean up old events (keep only last N per L2)
    pub async fn cleanup_old_events(&self, keep_count: usize) {
        let mut events = self.anchor_events.write().await;
        for l2_events in events.values_mut() {
            if l2_events.len() > keep_count {
                l2_events.drain(0..l2_events.len() - keep_count);
            }
        }
    }
}

impl Default for L2AnchorHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// External anchor manager
pub struct ExternalAnchorManager {
    /// L2 anchor handler
    l2_handler: L2AnchorHandler,
    /// External anchors
    external_anchors: Arc<RwLock<HashMap<String, ExternalAnchorData>>>,
}

impl ExternalAnchorManager {
    /// Create a new external anchor manager
    pub fn new() -> Self {
        Self {
            l2_handler: L2AnchorHandler::new(),
            external_anchors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get the L2 anchor handler
    pub fn l2_handler(&self) -> &L2AnchorHandler {
        &self.l2_handler
    }
    
    /// Add an external anchor
    pub async fn add_external_anchor(&self, anchor: ExternalAnchorData) {
        let chain_id = anchor.chain_id.clone();
        let mut anchors = self.external_anchors.write().await;
        anchors.insert(chain_id.clone(), anchor);
        info!("Added external anchor for chain {}", chain_id);
    }
    
    /// Get external anchor by chain ID
    pub async fn get_external_anchor(&self, chain_id: &str) -> Option<ExternalAnchorData> {
        let anchors = self.external_anchors.read().await;
        anchors.get(chain_id).cloned()
    }
    
    /// Get all external anchors
    pub async fn get_all_external_anchors(&self) -> Vec<ExternalAnchorData> {
        let anchors = self.external_anchors.read().await;
        anchors.values().cloned().collect()
    }

    /// Get metrics for monitoring
    pub async fn get_metrics(&self) -> AnchorMetrics {
        let l2_events = self.l2_handler.get_all_events().await;
        let external_anchors = self.external_anchors.read().await;
        
        let total_l2_events: u64 = l2_events.values().map(|events| events.len() as u64).sum();
        let total_external_anchors = external_anchors.len() as u64;
        let events_per_l2: HashMap<String, u64> = l2_events
            .into_iter()
            .map(|(l2_id, events)| (l2_id, events.len() as u64))
            .collect();
        
        AnchorMetrics {
            total_l2_events,
            total_external_anchors,
            events_per_l2,
        }
    }
}

impl Default for ExternalAnchorManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics for anchor events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMetrics {
    /// Total L2 anchor events
    pub total_l2_events: u64,
    /// Total external anchors
    pub total_external_anchors: u64,
    /// Events per L2
    pub events_per_l2: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosschain::types::{L2CommitTx, ProofType, DataAvailabilityMode};
    
    #[tokio::test]
    async fn test_l2_anchor_handler() {
        let handler = L2AnchorHandler::new();
        
        // Create a test commit
        let commit = L2CommitTx {
            l2_id: "test-l2".to_string(),
            epoch: 1,
            state_root: [0u8; 32],
            da_hash: [0u8; 32],
            proof_type: ProofType::ZkGroth16,
            proof: vec![1, 2, 3],
            inline_data: None,
        };
        
        // Handle the commit
        let event = handler.handle_l2_commit(&commit, 1000).await.unwrap();
        
        // Verify the event was created
        assert_eq!(event.l2_id, "test-l2");
        assert_eq!(event.epoch, 1);
        
        // Get events for the L2
        let events = handler.get_l2_events("test-l2").await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].epoch, 1);
    }
}
