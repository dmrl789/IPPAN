//! IPPAN RPC entry point - exposes the HTTP/JSON interface that wraps node
//! state, networking (`ippan_p2p`), and L2 configuration. Provides the `start_server`
//! helper plus shared types for peer snapshots consumed by explorers and tooling.
//!
pub mod server;
pub use server::{start_server, AppState, L2Config};

// Re-export types from ippan_p2p for convenience
pub use ippan_p2p::{HttpP2PNetwork, NetworkMessage, P2PConfig, P2PError, PeerInfo as P2PPeerInfo};

use serde::{Deserialize, Serialize};

/// Peer information for RPC responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub last_seen: u64,
    pub is_connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_info_serialization() {
        let peer = PeerInfo {
            id: "test-peer".to_string(),
            address: "http://localhost:9000".to_string(),
            last_seen: 123456,
            is_connected: true,
        };

        let serialized = serde_json::to_string(&peer).unwrap();
        let deserialized: PeerInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(peer.id, deserialized.id);
        assert_eq!(peer.address, deserialized.address);
        assert_eq!(peer.last_seen, deserialized.last_seen);
        assert_eq!(peer.is_connected, deserialized.is_connected);
    }

    #[test]
    fn test_peer_info_json_format() {
        let peer = PeerInfo {
            id: "node-1".to_string(),
            address: "http://192.168.1.1:9000".to_string(),
            last_seen: 1234567890,
            is_connected: false,
        };

        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("\"id\":\"node-1\""));
        assert!(json.contains("\"address\":\"http://192.168.1.1:9000\""));
        assert!(json.contains("\"last_seen\":1234567890"));
        assert!(json.contains("\"is_connected\":false"));
    }

    #[test]
    fn test_peer_info_defaults() {
        let peer = PeerInfo {
            id: String::new(),
            address: String::new(),
            last_seen: 0,
            is_connected: false,
        };

        assert_eq!(peer.id, "");
        assert_eq!(peer.address, "");
        assert_eq!(peer.last_seen, 0);
        assert!(!peer.is_connected);
    }
}
