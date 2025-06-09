use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub address: String, // e.g. "127.0.0.1:3031"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerList {
    pub peers: Vec<PeerInfo>,
}

pub struct P2P {
    pub peers: Arc<Mutex<Vec<PeerInfo>>>,
}

impl P2P {
    pub fn new(initial_peers: Vec<String>, my_addr: String) -> Self {
        let mut all = vec![PeerInfo { address: my_addr }];
        all.extend(initial_peers.into_iter().map(|a| PeerInfo { address: a }));
        Self { peers: Arc::new(Mutex::new(all)) }
    }

    pub fn add_peer(&self, address: &str) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.iter().any(|p| p.address == address) {
            peers.push(PeerInfo { address: address.to_string() });
        }
    }

    pub fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.lock().unwrap().clone()
    }

    // Fetch peers from another node and merge with ours
    pub async fn sync_peers(&self) {
        let peers = self.get_peers();
        for p in &peers {
            if let Ok(resp) = reqwest::get(format!("http://{}/peers", p.address)).await {
                if let Ok(list) = resp.json::<PeerList>().await {
                    for np in list.peers {
                        self.add_peer(&np.address);
                    }
                }
            }
        }
    }
}
