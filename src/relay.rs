use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::sync::{Arc, Mutex};
use crate::blockchain::{Block, Transaction};

#[derive(Clone)]
pub struct Relay {
    pub peers: Arc<Mutex<Vec<String>>>, // List of peer URLs (http://ip:port)
    pub client: Client,
}

impl Relay {
    pub fn new(peer_urls: Vec<String>) -> Self {
        Self {
            peers: Arc::new(Mutex::new(peer_urls)),
            client: Client::new(),
        }
    }

    // Relay a new block to all peers
    pub async fn relay_block(&self, block: &Block) {
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            let url = format!("http://{}/new_block", peer);
            let block = block.clone();
            let client = self.client.clone();
            tokio::spawn(async move {
                let _ = client.post(&url).json(&block).send().await;
            });
        }
    }

    // Relay a new transaction to all peers
    pub async fn relay_transaction(&self, tx: &Transaction) {
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            let url = format!("http://{}/new_tx", peer);
            let tx = tx.clone();
            let client = self.client.clone();
            tokio::spawn(async move {
                let _ = client.post(&url).json(&tx).send().await;
            });
        }
    }

    // Add a new peer (if not already present)
    pub fn add_peer(&self, url: &str) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.iter().any(|p| p == url) {
            peers.push(url.to_string());
        }
    }

    // Remove a peer (optional)
    pub fn remove_peer(&self, url: &str) {
        let mut peers = self.peers.lock().unwrap();
        peers.retain(|p| p != url);
    }
}
