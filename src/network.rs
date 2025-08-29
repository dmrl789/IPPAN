use crate::block::Block;
use crate::crypto::Hash;
use crate::error::{Error, Result};
use crate::transaction::Transaction;
use futures::StreamExt;
use libp2p::{
    core::upgrade,
    gossipsub::{self, Gossipsub, GossipsubConfig, GossipsubEvent, GossipsubMessage, ValidationMode},
    identify, kad, mdns, ping,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    PeerId, Transport,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

// Network topics
pub const TX_GOSSIP_TOPIC: &str = "ippan-tx-gossip";
pub const BLOCK_GOSSIP_TOPIC: &str = "ippan-block-gossip";
pub const ROUND_GOSSIP_TOPIC: &str = "ippan-round-gossip";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Transaction(Transaction),
    Block(Block),
    RoundData(Vec<u8>), // Serialized round data
    TimeSync(u64),      // IPPAN time sync
}

#[derive(NetworkBehaviour)]
pub struct IppanBehaviour {
    gossipsub: Gossipsub<'static>,
    identify: identify::Behaviour,
    kad: kad::Kademlia<kad::store::MemoryStore>,
    mdns: mdns::tokio::Behaviour,
    ping: ping::Behaviour,
}

pub struct NetworkManager {
    swarm: Swarm<IppanBehaviour>,
    tx_sender: mpsc::UnboundedSender<NetworkMessage>,
    tx_receiver: mpsc::UnboundedReceiver<NetworkMessage>,
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    metrics: Arc<crate::metrics::Metrics>,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<String>,
    pub last_seen: std::time::Instant,
    pub is_connected: bool,
}

impl NetworkManager {
    pub async fn new(
        keypair: libp2p::identity::Keypair,
        metrics: Arc<crate::metrics::Metrics>,
    ) -> Result<Self, Error> {
        let peer_id = PeerId::from(keypair.public());
        info!("Starting network with peer ID: {}", peer_id);

        // Create transport
        let transport = libp2p::tokio::Transport::new(
            libp2p::core::upgrade::Version::V1,
            libp2p::noise::NoiseAuthenticated::xx(&keypair).unwrap(),
            libp2p::yamux::YamuxConfig::default(),
        )?
        .boxed();

        // Create gossipsub config
        let gossipsub_config = GossipsubConfig::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .max_transmit_size(2 * 1024 * 1024) // 2MB
            .duplicate_cache_time(Duration::from_secs(60))
            .mesh_n(6)
            .mesh_n_low(4)
            .mesh_n_high(12)
            .gossip_lazy(6)
            .fanout_ttl(Duration::from_secs(60))
            .prune_peers(64)
            .prune_backoff(Duration::from_secs(1));

        // Create gossipsub
        let mut gossipsub = Gossipsub::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )?;

        // Subscribe to topics
        gossipsub.subscribe(&gossipsub::IdentTopic::new(TX_GOSSIP_TOPIC))?;
        gossipsub.subscribe(&gossipsub::IdentTopic::new(BLOCK_GOSSIP_TOPIC))?;
        gossipsub.subscribe(&gossipsub::IdentTopic::new(ROUND_GOSSIP_TOPIC))?;

        // Create identify behaviour
        let identify = identify::Behaviour::new(identify::Config::new(
            "/ippan/1.0.0".to_string(),
            keypair.public(),
        ));

        // Create Kademlia DHT
        let kad_store = kad::store::MemoryStore::new(peer_id);
        let kad = kad::Kademlia::new(peer_id, kad_store);

        // Create mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default())?;

        // Create ping behaviour
        let ping = ping::Behaviour::new(ping::Config::default());

        // Create network behaviour
        let behaviour = IppanBehaviour {
            gossipsub,
            identify,
            kad,
            mdns,
            ping,
        };

        // Create swarm
        let swarm = Swarm::new(transport, behaviour, peer_id);

        // Create message channels
        let (tx_sender, tx_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            swarm,
            tx_sender,
            tx_receiver,
            peers: Arc::new(RwLock::new(HashMap::new())),
            metrics,
        })
    }

    pub async fn start(&mut self, listen_addr: &str) -> Result<(), Error> {
        let addr = listen_addr.parse()?;
        self.swarm.listen_on(addr)?;
        info!("Listening on {}", listen_addr);

        // Start network event loop
        self.run_event_loop().await;
        Ok(())
    }

    async fn run_event_loop(&mut self) {
        loop {
            tokio::select! {
                swarm_event = self.swarm.next() => {
                    if let Some(event) = swarm_event {
                        self.handle_swarm_event(event).await;
                    }
                }
                message = self.tx_receiver.recv() => {
                    if let Some(msg) = message {
                        self.handle_internal_message(msg).await;
                    }
                }
            }
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<IppanBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(IppanBehaviourEvent::Gossipsub(GossipsubEvent::Message {
                propagation_source,
                message_id,
                message,
            })) => {
                self.handle_gossipsub_message(propagation_source, message_id, message).await;
            }
            SwarmEvent::Behaviour(IppanBehaviourEvent::Gossipsub(GossipsubEvent::Subscribed {
                peer_id,
                topic,
            })) => {
                info!("Peer {} subscribed to topic {}", peer_id, topic);
            }
            SwarmEvent::Behaviour(IppanBehaviourEvent::Gossipsub(GossipsubEvent::Unsubscribed {
                peer_id,
                topic,
            })) => {
                info!("Peer {} unsubscribed from topic {}", peer_id, topic);
            }
            SwarmEvent::Behaviour(IppanBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
            })) => {
                info!("Received identify info from peer {}: {:?}", peer_id, info);
                self.update_peer_info(peer_id, info).await;
            }
            SwarmEvent::Behaviour(IppanBehaviourEvent::Kad(kad::Event::OutboundQueryCompleted {
                result,
                ..
            })) => {
                match result {
                    kad::QueryResult::Bootstrap(Ok(peers)) => {
                        info!("Bootstrap completed with {} peers", peers.len());
                        for (peer_id, _) in peers {
                            self.add_peer(peer_id).await;
                        }
                    }
                    kad::QueryResult::GetProviders(Ok(providers)) => {
                        info!("Found {} providers", providers.len());
                        for (peer_id, _) in providers {
                            self.add_peer(peer_id).await;
                        }
                    }
                    _ => {}
                }
            }
            SwarmEvent::Behaviour(IppanBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, _) in list {
                    info!("mDNS discovered peer: {}", peer_id);
                    self.add_peer(peer_id).await;
                }
            }
            SwarmEvent::Behaviour(IppanBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _) in list {
                    info!("mDNS expired peer: {}", peer_id);
                    self.remove_peer(peer_id).await;
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.add_peer(peer_id).await;
                self.metrics.update_peers_connected(self.get_peer_count().await);
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.remove_peer(peer_id).await;
                self.metrics.update_peers_connected(self.get_peer_count().await);
            }
            _ => {}
        }
    }

    async fn handle_gossipsub_message(
        &self,
        _source: PeerId,
        _message_id: gossipsub::MessageId,
        message: GossipsubMessage,
    ) {
        self.metrics.record_message_received();

        match message.topic.as_str() {
            TX_GOSSIP_TOPIC => {
                if let Ok(tx) = bincode::deserialize::<Transaction>(&message.data) {
                    debug!("Received transaction via gossip: {:?}", tx.compute_id());
                    // Handle transaction
                }
            }
            BLOCK_GOSSIP_TOPIC => {
                if let Ok(block) = bincode::deserialize::<Block>(&message.data) {
                    debug!("Received block via gossip: {:?}", block.compute_id());
                    // Handle block
                }
            }
            ROUND_GOSSIP_TOPIC => {
                debug!("Received round data via gossip");
                // Handle round data
            }
            _ => {
                warn!("Unknown topic: {}", message.topic);
            }
        }
    }

    async fn handle_internal_message(&self, message: NetworkMessage) {
        match message {
            NetworkMessage::Transaction(tx) => {
                self.publish_transaction(tx).await;
            }
            NetworkMessage::Block(block) => {
                self.publish_block(block).await;
            }
            NetworkMessage::RoundData(data) => {
                self.publish_round_data(data).await;
            }
            NetworkMessage::TimeSync(time) => {
                self.publish_time_sync(time).await;
            }
        }
    }

    pub async fn publish_transaction(&self, tx: Transaction) {
        if let Ok(data) = bincode::serialize(&tx) {
            let topic = gossipsub::IdentTopic::new(TX_GOSSIP_TOPIC);
            if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                error!("Failed to publish transaction: {}", e);
            } else {
                self.metrics.record_message_sent();
                debug!("Published transaction: {:?}", tx.compute_id());
            }
        }
    }

    pub async fn publish_block(&self, block: Block) {
        if let Ok(data) = bincode::serialize(&block) {
            let topic = gossipsub::IdentTopic::new(BLOCK_GOSSIP_TOPIC);
            if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                error!("Failed to publish block: {}", e);
            } else {
                self.metrics.record_message_sent();
                debug!("Published block: {:?}", block.compute_id());
            }
        }
    }

    pub async fn publish_round_data(&self, data: Vec<u8>) {
        let topic = gossipsub::IdentTopic::new(ROUND_GOSSIP_TOPIC);
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
            error!("Failed to publish round data: {}", e);
        } else {
            self.metrics.record_message_sent();
            debug!("Published round data");
        }
    }

    pub async fn publish_time_sync(&self, time: u64) {
        if let Ok(data) = bincode::serialize(&NetworkMessage::TimeSync(time)) {
            let topic = gossipsub::IdentTopic::new(ROUND_GOSSIP_TOPIC);
            if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                error!("Failed to publish time sync: {}", e);
            } else {
                self.metrics.record_message_sent();
                debug!("Published time sync: {}", time);
            }
        }
    }

    pub async fn add_bootstrap_peer(&mut self, peer_addr: &str) -> Result<(), Error> {
        let addr = peer_addr.parse()?;
        self.swarm.add_external_address(addr);
        info!("Added bootstrap peer: {}", peer_addr);
        Ok(())
    }

    pub async fn bootstrap(&mut self) {
        info!("Starting DHT bootstrap...");
        self.swarm.behaviour_mut().kad.bootstrap().unwrap();
    }

    async fn add_peer(&self, peer_id: PeerId) {
        let mut peers = self.peers.write().await;
        peers.insert(peer_id, PeerInfo {
            peer_id,
            addresses: Vec::new(),
            last_seen: std::time::Instant::now(),
            is_connected: true,
        });
    }

    async fn remove_peer(&self, peer_id: PeerId) {
        let mut peers = self.peers.write().await;
        if let Some(peer_info) = peers.get_mut(&peer_id) {
            peer_info.is_connected = false;
        }
    }

    async fn update_peer_info(&self, peer_id: PeerId, info: identify::Info) {
        let mut peers = self.peers.write().await;
        if let Some(peer_info) = peers.get_mut(&peer_id) {
            peer_info.addresses = info.listen_addrs.iter().map(|addr| addr.to_string()).collect();
            peer_info.last_seen = std::time::Instant::now();
        }
    }

    pub async fn get_peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.values().filter(|p| p.is_connected).count()
    }

    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    pub fn get_message_sender(&self) -> mpsc::UnboundedSender<NetworkMessage> {
        self.tx_sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let keypair = identity::Keypair::generate_ed25519();
        let metrics = Arc::new(crate::metrics::Metrics::new(1));
        
        let network = NetworkManager::new(keypair, metrics).await;
        assert!(network.is_ok());
    }

    #[tokio::test]
    async fn test_network_message_serialization() {
        let message = NetworkMessage::TimeSync(1234567890);
        let serialized = bincode::serialize(&message).unwrap();
        let deserialized: NetworkMessage = bincode::deserialize(&serialized).unwrap();
        
        match deserialized {
            NetworkMessage::TimeSync(time) => assert_eq!(time, 1234567890),
            _ => panic!("Unexpected message type"),
        }
    }
}
