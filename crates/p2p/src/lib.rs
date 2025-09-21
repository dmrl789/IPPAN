use anyhow::Result;
use ippan_types::{Block, Transaction};
use std::collections::HashSet;

/// P2P network interface for IPPAN
pub trait P2PNetwork {
    /// Start the P2P network
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the P2P network
    async fn stop(&mut self) -> Result<()>;
    
    /// Broadcast a block to all peers
    async fn broadcast_block(&self, block: &Block) -> Result<()>;
    
    /// Broadcast a transaction to all peers
    async fn broadcast_transaction(&self, tx: &Transaction) -> Result<()>;
    
    /// Get connected peers
    fn get_peers(&self) -> Vec<PeerInfo>;
}

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub is_connected: bool,
}

/// In-memory P2P network implementation (for testing/development)
pub struct MemoryP2PNetwork {
    peers: HashSet<String>,
    is_running: bool,
}

impl MemoryP2PNetwork {
    pub fn new() -> Self {
        Self {
            peers: HashSet::new(),
            is_running: false,
        }
    }
}

impl P2PNetwork for MemoryP2PNetwork {
    async fn start(&mut self) -> Result<()> {
        self.is_running = true;
        tracing::info!("P2P network started (memory implementation)");
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        self.is_running = false;
        tracing::info!("P2P network stopped");
        Ok(())
    }
    
    async fn broadcast_block(&self, block: &Block) -> Result<()> {
        if self.is_running {
            tracing::info!("Broadcasting block {} to {} peers", 
                hex::encode(block.hash()), self.peers.len());
        }
        Ok(())
    }
    
    async fn broadcast_transaction(&self, tx: &Transaction) -> Result<()> {
        if self.is_running {
            tracing::info!("Broadcasting transaction {} to {} peers", 
                hex::encode(tx.hash()), self.peers.len());
        }
        Ok(())
    }
    
    fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.iter().map(|id| PeerInfo {
            id: id.clone(),
            address: format!("{}:3000", id),
            is_connected: true,
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{Block, Transaction};

    #[tokio::test]
    async fn test_p2p_network() {
        let mut network = MemoryP2PNetwork::new();
        
        // Test starting and stopping
        network.start().await.unwrap();
        assert!(network.is_running);
        
        network.stop().await.unwrap();
        assert!(!network.is_running);
    }
}
