use std::collections::HashMap;

/// Metadata describing a connected peer in the network directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub id: Option<String>,
    pub address: String,
    pub connected: bool,
    pub last_seen: Option<u64>,
    pub first_connected: Option<u64>,
}

impl Peer {
    pub fn new<S: Into<String>>(address: S) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: None,
            address: address.into(),
            connected: true,
            last_seen: Some(now),
            first_connected: Some(now),
        }
    }

    pub fn with_id<I: Into<String>, A: Into<String>>(id: I, address: A) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Some(id.into()),
            address: address.into(),
            connected: true,
            last_seen: Some(now),
            first_connected: Some(now),
        }
    }

    /// Update the last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> Option<u64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.first_connected.map(|first| now.saturating_sub(first))
    }
}

/// In-memory directory that keeps track of the known peers and their
/// connection status.
#[derive(Debug, Default)]
pub struct PeerDirectory {
    peers: HashMap<String, Peer>,
}

impl PeerDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_peers<I: IntoIterator<Item = Peer>>(peers: I) -> Self {
        let mut directory = Self::default();
        for peer in peers {
            directory.upsert_peer(peer);
        }
        directory
    }

    pub fn upsert_peer(&mut self, peer: Peer) {
        self.peers.insert(peer.address.clone(), peer);
    }

    pub fn mark_connected<S: AsRef<str>>(&mut self, address: S, connected: bool) {
        if let Some(peer) = self.peers.get_mut(address.as_ref()) {
            peer.connected = connected;
            if connected {
                peer.update_last_seen();
            }
        }
    }

    pub fn update_last_seen<S: AsRef<str>>(&mut self, address: S) {
        if let Some(peer) = self.peers.get_mut(address.as_ref()) {
            peer.update_last_seen();
        }
    }

    pub fn get_peer<S: AsRef<str>>(&self, address: S) -> Option<&Peer> {
        self.peers.get(address.as_ref())
    }

    pub fn list_connected(&self) -> Vec<Peer> {
        self.peers
            .values()
            .filter(|peer| peer.connected)
            .cloned()
            .collect()
    }

    pub fn list_all(&self) -> Vec<Peer> {
        self.peers.values().cloned().collect()
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn directory_filters_connected_peers() {
        let mut directory = PeerDirectory::new();
        directory.upsert_peer(Peer::new("127.0.0.1:9000"));
        let mut disconnected = Peer::with_id("node-b", "192.168.1.5:9000");
        disconnected.connected = false;
        directory.upsert_peer(disconnected.clone());

        let connected = directory.list_connected();
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0].address, "127.0.0.1:9000");

        directory.mark_connected(&disconnected.address, true);
        let connected = directory.list_connected();
        assert_eq!(connected.len(), 2);
    }
}
