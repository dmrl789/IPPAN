use std::collections::HashMap;

/// Metadata describing a connected peer in the network directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub id: Option<String>,
    pub address: String,
    pub connected: bool,
}

impl Peer {
    pub fn new<S: Into<String>>(address: S) -> Self {
        Self {
            id: None,
            address: address.into(),
            connected: true,
        }
    }

    pub fn with_id<I: Into<String>, A: Into<String>>(id: I, address: A) -> Self {
        Self {
            id: Some(id.into()),
            address: address.into(),
            connected: true,
        }
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
        }
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

#[cfg(test)]
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
