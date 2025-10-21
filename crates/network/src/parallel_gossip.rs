use std::sync::Arc;
use std::time::Duration;

use futures::{future::join_all, stream, StreamExt};
use ippan_types::{Block, IppanTimeMicros};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::warn;

use crate::peers::PeerDirectory;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum GossipMessage {
    Block(Box<Block>),
    Transactions { id: String, txs: Vec<Value> },
}

/// Parallel gossip relay for block and transaction propagation.
/// Each broadcast batch is sent concurrently to all connected peers.
#[derive(Clone)]
pub struct ParallelGossip {
    peers: Arc<RwLock<PeerDirectory>>,
    connect_timeout: Duration,
}

impl ParallelGossip {
    pub fn new(peers: Arc<RwLock<PeerDirectory>>) -> Self {
        Self {
            peers,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        }
    }

    pub fn with_timeout(peers: Arc<RwLock<PeerDirectory>>, connect_timeout: Duration) -> Self {
        Self {
            peers,
            connect_timeout,
        }
    }

    pub async fn broadcast_blocks_parallel(&self, blocks: Vec<Block>) {
        if blocks.is_empty() {
            return;
        }

        let peers_snapshot = {
            let peers = self.peers.read().await;
            peers.list_connected()
        };

        if peers_snapshot.is_empty() {
            return;
        }

        let peers_snapshot = Arc::new(peers_snapshot);
        let timeout = self.connect_timeout;

        stream::iter(blocks)
            .for_each_concurrent(None, |block| {
                let peers_snapshot = peers_snapshot.clone();
                async move {
                    let payload = match serde_json::to_vec(&GossipMessage::Block(Box::new(block))) {
                        Ok(bytes) => Arc::<[u8]>::from(bytes),
                        Err(err) => {
                            warn!(
                                target = "parallel_gossip",
                                error = %err,
                                "failed to serialize block gossip message",
                            );
                            return;
                        }
                    };

                    let send_tasks = peers_snapshot.iter().map(|peer| {
                        let addr = peer.address.clone();
                        let payload = payload.clone();
                        async move {
                            if let Err(error) =
                                Self::send_to_peer(addr.clone(), payload.clone(), timeout).await
                            {
                                warn!(
                                    target = "parallel_gossip",
                                    address = %addr,
                                    error = %error,
                                    "failed to send block gossip",
                                );
                            }
                        }
                    });

                    join_all(send_tasks).await;
                }
            })
            .await;
    }

    pub async fn broadcast_transactions_parallel<T>(&self, txs: Vec<T>)
    where
        T: Serialize + Send + Sync + Clone,
    {
        if txs.is_empty() {
            return;
        }

        let peers_snapshot = {
            let peers = self.peers.read().await;
            peers.list_connected()
        };

        if peers_snapshot.is_empty() {
            return;
        }

        let peers_snapshot = Arc::new(peers_snapshot);
        let timeout = self.connect_timeout;
        let chunks: Vec<Vec<T>> = txs.chunks(200).map(|chunk| chunk.to_vec()).collect();

        stream::iter(chunks)
            .for_each_concurrent(None, |batch| {
                let peers_snapshot = peers_snapshot.clone();
                async move {
                    let gossip_id = IppanTimeMicros::now().0.to_string();
                    let txs: Vec<Value> = batch
                        .iter()
                        .filter_map(|tx| match serde_json::to_value(tx) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                warn!(
                                    target = "parallel_gossip",
                                    error = %err,
                                    "failed to serialize transaction for gossip",
                                );
                                None
                            }
                        })
                        .collect();

                    if txs.is_empty() {
                        return;
                    }

                    let payload = match serde_json::to_vec(&GossipMessage::Transactions {
                        id: gossip_id,
                        txs,
                    }) {
                        Ok(bytes) => Arc::<[u8]>::from(bytes),
                        Err(err) => {
                            warn!(
                                target = "parallel_gossip",
                                error = %err,
                                "failed to serialize transaction batch gossip message",
                            );
                            return;
                        }
                    };

                    let send_tasks = peers_snapshot.iter().map(|peer| {
                        let addr = peer.address.clone();
                        let payload = payload.clone();
                        async move {
                            if let Err(error) =
                                Self::send_to_peer(addr.clone(), payload.clone(), timeout).await
                            {
                                warn!(
                                    target = "parallel_gossip",
                                    address = %addr,
                                    error = %error,
                                    "failed to send transaction gossip",
                                );
                            }
                        }
                    });

                    join_all(send_tasks).await;
                }
            })
            .await;
    }

    async fn send_to_peer(
        addr: String,
        payload: Arc<[u8]>,
        timeout_duration: Duration,
    ) -> Result<(), String> {
        match timeout(timeout_duration, TcpStream::connect(&addr)).await {
            Ok(Ok(mut stream)) => {
                if let Err(err) = stream.write_all(&payload).await {
                    return Err(format!("write error {err}"));
                }
                Ok(())
            }
            Ok(Err(err)) => Err(format!("connect error {err}")),
            Err(_) => Err("timeout".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peers::PeerDirectory;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn empty_inputs_exit_early() {
        let directory = Arc::new(RwLock::new(PeerDirectory::new()));
        let gossip = ParallelGossip::new(directory);

        gossip.broadcast_blocks_parallel(Vec::new()).await;
        gossip
            .broadcast_transactions_parallel::<serde_json::Value>(Vec::new())
            .await;
    }
}
