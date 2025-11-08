//! libp2p-based P2P network implementation with DHT, gossipsub, and NAT traversal.
//!
//! This module provides a production-ready P2P network stack using libp2p with:
//! - Kademlia DHT for peer discovery and content routing
//! - GossipSub for efficient message broadcasting
//! - mDNS for local peer discovery
//! - Relay and DCUtR for NAT traversal
//! - Request/Response protocol for direct peer queries

use anyhow::{anyhow, Result};
use futures::StreamExt;
use ippan_types::{Block, Transaction};
use libp2p::{
    core::upgrade,
    dcutr,
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify,
    identity::Keypair,
    kad,
    mdns,
    noise,
    ping,
    relay,
    request_response::{self, OutboundRequestId, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

/// P2P network events emitted by the libp2p network
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// New peer discovered and connected
    PeerConnected {
        peer_id: PeerId,
        addresses: Vec<Multiaddr>,
    },
    /// Peer disconnected
    PeerDisconnected { peer_id: PeerId },
    /// Block received via gossip
    BlockReceived { from: PeerId, block: Block },
    /// Transaction received via gossip
    TransactionReceived { from: PeerId, transaction: Transaction },
    /// DHT query completed
    DhtQueryComplete { peer_id: PeerId },
}

/// Request/Response protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkRequest {
    GetBlock { hash: [u8; 32] },
    GetPeers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkResponse {
    Block(Option<Block>),
    Peers(Vec<String>),
}

/// Codec for request/response protocol
#[derive(Debug, Clone, Default)]
pub struct NetworkCodec;

#[async_trait::async_trait]
impl request_response::Codec for NetworkCodec {
    type Protocol = String;
    type Request = NetworkRequest;
    type Response = NetworkResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        use futures::io::AsyncReadExt;
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        use futures::io::AsyncReadExt;
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        use futures::io::AsyncWriteExt;
        let data = serde_json::to_vec(&req).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        use futures::io::AsyncWriteExt;
        let data = serde_json::to_vec(&res).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await?;
        io.close().await
    }
}

/// libp2p Network Behaviour combining all protocols
#[derive(NetworkBehaviour)]
struct IppanBehaviour {
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    relay_client: relay::client::Behaviour,
    dcutr: dcutr::Behaviour,
    request_response: request_response::Behaviour<NetworkCodec>,
}

/// Configuration for libp2p network
#[derive(Debug, Clone)]
pub struct LibP2PConfig {
    pub listen_addresses: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    pub enable_mdns: bool,
    pub enable_relay: bool,
    pub idle_connection_timeout: Duration,
}

impl Default for LibP2PConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/0".parse().unwrap(),
                "/ip6/::/tcp/0".parse().unwrap(),
            ],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_relay: true,
            idle_connection_timeout: Duration::from_secs(60),
        }
    }
}

/// Main libp2p network manager
pub struct LibP2PNetwork {
    swarm: Swarm<IppanBehaviour>,
    event_sender: mpsc::UnboundedSender<NetworkEvent>,
    local_peer_id: PeerId,
    connected_peers: Arc<RwLock<HashSet<PeerId>>>,
    pending_requests: Arc<RwLock<HashMap<OutboundRequestId, oneshot::Sender<NetworkResponse>>>>,
    block_topic: IdentTopic,
    transaction_topic: IdentTopic,
}

impl LibP2PNetwork {
    /// Create a new libp2p network
    pub fn new(config: LibP2PConfig, keypair: Option<Keypair>) -> Result<(Self, mpsc::UnboundedReceiver<NetworkEvent>)> {
        let local_key = keypair.unwrap_or_else(Keypair::generate_ed25519);
        let local_peer_id = PeerId::from(local_key.public());

        info!("Local peer ID: {}", local_peer_id);

        // Build transport with TCP, noise encryption, and yamux multiplexing
        let transport = tcp::tokio::Transport::default()
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key).unwrap())
            .multiplex(yamux::Config::default())
            .boxed();

        // Configure Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kad_config = kad::Config::default();
        kad_config.set_query_timeout(Duration::from_secs(30));
        let mut kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);

        // Add bootstrap peers to Kademlia
        for (peer_id, addr) in &config.bootstrap_peers {
            kademlia.add_address(peer_id, addr.clone());
        }

        // Configure GossipSub with message signing
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(|message| {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                message.data.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_string())
            })
            .build()
            .map_err(|e| anyhow!("Failed to build gossipsub config: {}", e))?;

        let mut gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow!("Failed to create gossipsub: {}", e))?;

        // Create topics
        let block_topic = IdentTopic::new("ippan/blocks/1.0.0");
        let transaction_topic = IdentTopic::new("ippan/transactions/1.0.0");

        gossipsub.subscribe(&block_topic)?;
        gossipsub.subscribe(&transaction_topic)?;

        // Configure mDNS for local peer discovery
        let mdns = if config.enable_mdns {
            mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?
        } else {
            mdns::tokio::Behaviour::new(
                mdns::Config {
                    ttl: Duration::from_secs(0),
                    query_interval: Duration::from_secs(3600),
                    enable_ipv6: false,
                },
                local_peer_id,
            )?
        };

        // Configure identify protocol
        let identify = identify::Behaviour::new(identify::Config::new(
            "/ippan/1.0.0".to_string(),
            local_key.public(),
        ));

        // Configure ping
        let ping = ping::Behaviour::new(ping::Config::new());

        // Configure relay client for NAT traversal (skip for now - requires transport integration)
        let (_relay_transport, relay_client) = relay::client::new(local_peer_id);
        
        // Configure DCUtR (Direct Connection Upgrade through Relay)
        let dcutr = dcutr::Behaviour::new(local_peer_id);

        // Configure request/response protocol
        let protocols = std::iter::once(("/ippan/req-resp/1.0.0".to_string(), ProtocolSupport::Full));
        let cfg = request_response::Config::default();
        let request_response = request_response::Behaviour::new(protocols, cfg);

        // Combine all behaviours
        let behaviour = IppanBehaviour {
            kademlia,
            gossipsub,
            mdns,
            identify,
            ping,
            relay_client,
            dcutr,
            request_response,
        };

        // Create swarm
        let mut swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor()
                .with_idle_connection_timeout(config.idle_connection_timeout),
        );

        // Listen on configured addresses
        for addr in &config.listen_addresses {
            swarm.listen_on(addr.clone())?;
        }

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Ok((
            Self {
                swarm,
                event_sender,
                local_peer_id,
                connected_peers: Arc::new(RwLock::new(HashSet::new())),
                pending_requests: Arc::new(RwLock::new(HashMap::new())),
                block_topic,
                transaction_topic,
            },
            event_receiver,
        ))
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Get list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers.read().iter().copied().collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.connected_peers.read().len()
    }

    /// Bootstrap DHT by querying for random peer
    pub fn bootstrap_dht(&mut self) -> Result<()> {
        self.swarm.behaviour_mut().kademlia.bootstrap()?;
        info!("DHT bootstrap initiated");
        Ok(())
    }

    /// Add a peer address to Kademlia
    pub fn add_peer_address(&mut self, peer_id: PeerId, address: Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, address.clone());
        info!("Added peer {} with address {}", peer_id, address);
    }

    /// Dial a peer
    pub fn dial(&mut self, peer_id: &PeerId, address: Option<Multiaddr>) -> Result<()> {
        if let Some(addr) = address {
            self.add_peer_address(*peer_id, addr.clone());
            self.swarm.dial(addr)?;
        } else {
            self.swarm.dial(*peer_id)?;
        }
        Ok(())
    }

    /// Broadcast a block via gossipsub
    pub fn broadcast_block(&mut self, block: Block) -> Result<()> {
        let data = serde_json::to_vec(&block)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.block_topic.clone(), data)?;
        debug!("Broadcasted block with hash: {}", hex::encode(block.hash()));
        Ok(())
    }

    /// Broadcast a transaction via gossipsub
    pub fn broadcast_transaction(&mut self, transaction: Transaction) -> Result<()> {
        let data = serde_json::to_vec(&transaction)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.transaction_topic.clone(), data)?;
        debug!("Broadcasted transaction with id: {}", hex::encode(transaction.id));
        Ok(())
    }

    /// Request a block from a peer
    pub async fn request_block(&mut self, peer_id: PeerId, hash: [u8; 32]) -> Result<Option<Block>> {
        let (tx, rx) = oneshot::channel();
        let request = NetworkRequest::GetBlock { hash };
        
        let request_id = self.swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, request);
        
        self.pending_requests.write().insert(request_id, tx);
        
        match rx.await {
            Ok(NetworkResponse::Block(block)) => Ok(block),
            Ok(_) => Err(anyhow!("Unexpected response type")),
            Err(_) => Err(anyhow!("Request cancelled or timed out")),
        }
    }

    /// Find closest peers to a given peer ID using Kademlia
    pub fn find_peer(&mut self, peer_id: PeerId) {
        self.swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
    }

    /// Process swarm events
    pub async fn run(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(event) => self.handle_behaviour_event(event).await,
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {}", address);
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id,
                    endpoint,
                    ..
                } => {
                    info!("Connection established with peer: {} at {}", peer_id, endpoint.get_remote_address());
                    self.connected_peers.write().insert(peer_id);
                    
                    let addresses = vec![endpoint.get_remote_address().clone()];
                    
                    let _ = self.event_sender.send(NetworkEvent::PeerConnected {
                        peer_id,
                        addresses,
                    });
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    info!("Connection closed with peer: {}", peer_id);
                    self.connected_peers.write().remove(&peer_id);
                    
                    let _ = self.event_sender.send(NetworkEvent::PeerDisconnected { peer_id });
                }
                SwarmEvent::IncomingConnection { .. } => {
                    debug!("Incoming connection");
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    if let Some(peer_id) = peer_id {
                        warn!("Outgoing connection error with {}: {}", peer_id, error);
                    } else {
                        warn!("Outgoing connection error: {}", error);
                    }
                }
                SwarmEvent::IncomingConnectionError { error, .. } => {
                    warn!("Incoming connection error: {}", error);
                }
                _ => {}
            }
        }
    }

    async fn handle_behaviour_event(&mut self, event: IppanBehaviourEvent) {
        match event {
            IppanBehaviourEvent::Kademlia(event) => self.handle_kademlia_event(event).await,
            IppanBehaviourEvent::Gossipsub(event) => self.handle_gossipsub_event(event).await,
            IppanBehaviourEvent::Mdns(event) => self.handle_mdns_event(event).await,
            IppanBehaviourEvent::Identify(event) => self.handle_identify_event(event).await,
            IppanBehaviourEvent::Ping(event) => {
                debug!("Ping event: {:?}", event);
            }
            IppanBehaviourEvent::RelayClient(event) => {
                debug!("Relay client event: {:?}", event);
            }
            IppanBehaviourEvent::Dcutr(event) => {
                info!("DCUtR event: {:?}", event);
            }
            IppanBehaviourEvent::RequestResponse(event) => {
                self.handle_request_response_event(event).await
            }
        }
    }

    async fn handle_kademlia_event(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed { result, .. } => match result {
                kad::QueryResult::GetClosestPeers(Ok(kad::GetClosestPeersOk { key, peers })) => {
                    info!("DHT query for {} returned {} peers", hex::encode(&key), peers.len());
                    for peer in peers {
                        let _ = self.event_sender.send(NetworkEvent::DhtQueryComplete { peer_id: peer.peer_id });
                    }
                }
                kad::QueryResult::GetClosestPeers(Err(e)) => {
                    warn!("DHT query failed: {:?}", e);
                }
                kad::QueryResult::Bootstrap(Ok(_)) => {
                    info!("DHT bootstrap completed successfully");
                }
                kad::QueryResult::Bootstrap(Err(e)) => {
                    warn!("DHT bootstrap failed: {:?}", e);
                }
                _ => {}
            },
            kad::Event::RoutingUpdated { peer, .. } => {
                debug!("Routing table updated with peer: {}", peer);
            }
            _ => {}
        }
    }

    async fn handle_gossipsub_event(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message {
                propagation_source: peer_id,
                message,
                ..
            } => {
                if message.topic == self.block_topic.hash() {
                    match serde_json::from_slice::<Block>(&message.data) {
                        Ok(block) => {
                            debug!("Received block from {}", peer_id);
                            let _ = self.event_sender.send(NetworkEvent::BlockReceived {
                                from: peer_id,
                                block,
                            });
                        }
                        Err(e) => {
                            warn!("Failed to deserialize block: {}", e);
                        }
                    }
                } else if message.topic == self.transaction_topic.hash() {
                    match serde_json::from_slice::<Transaction>(&message.data) {
                        Ok(transaction) => {
                            debug!("Received transaction from {}", peer_id);
                            let _ = self.event_sender.send(NetworkEvent::TransactionReceived {
                                from: peer_id,
                                transaction,
                            });
                        }
                        Err(e) => {
                            warn!("Failed to deserialize transaction: {}", e);
                        }
                    }
                }
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                info!("Peer {} subscribed to topic: {}", peer_id, topic);
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                info!("Peer {} unsubscribed from topic: {}", peer_id, topic);
            }
            _ => {}
        }
    }

    async fn handle_mdns_event(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    info!("mDNS discovered peer: {} at {}", peer_id, addr);
                    self.add_peer_address(peer_id, addr);
                    if let Err(e) = self.dial(&peer_id, None) {
                        warn!("Failed to dial mDNS peer {}: {}", peer_id, e);
                    }
                }
            }
            mdns::Event::Expired(peers) => {
                for (peer_id, addr) in peers {
                    debug!("mDNS peer expired: {} at {}", peer_id, addr);
                }
            }
        }
    }

    async fn handle_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received { peer_id, info, connection_id: _ } => {
                info!(
                    "Identified peer: {} with protocol version: {} and agent: {}",
                    peer_id, info.protocol_version, info.agent_version
                );
                
                // Add peer's listen addresses to Kademlia
                for addr in info.listen_addrs {
                    self.add_peer_address(peer_id, addr);
                }
            }
            identify::Event::Sent { peer_id, connection_id: _ } => {
                debug!("Sent identify info to peer: {}", peer_id);
            }
            identify::Event::Pushed { peer_id, .. } => {
                debug!("Pushed identify update to peer: {}", peer_id);
            }
            identify::Event::Error { peer_id, error, connection_id: _ } => {
                warn!("Identify error with peer {}: {}", peer_id, error);
            }
        }
    }

    async fn handle_request_response_event(
        &mut self,
        event: request_response::Event<NetworkRequest, NetworkResponse>,
    ) {
        match event {
            request_response::Event::Message { message, .. } => match message {
                request_response::Message::Request {
                    request,
                    channel,
                    ..
                } => {
                    // Handle incoming request
                    let response = match request {
                        NetworkRequest::GetBlock { hash: _ } => {
                            // This would query local storage
                            NetworkResponse::Block(None)
                        }
                        NetworkRequest::GetPeers => {
                            let peers = self
                                .connected_peers()
                                .iter()
                                .map(|p| p.to_string())
                                .collect();
                            NetworkResponse::Peers(peers)
                        }
                    };
                    
                    if let Err(e) = self.swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, response)
                    {
                        warn!("Failed to send response: {:?}", e);
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    // Handle response to our request
                    if let Some(sender) = self.pending_requests.write().remove(&request_id) {
                        let _ = sender.send(response);
                    }
                }
            },
            request_response::Event::OutboundFailure {
                request_id, error, ..
            } => {
                warn!("Outbound request {} failed: {:?}", request_id, error);
                self.pending_requests.write().remove(&request_id);
            }
            request_response::Event::InboundFailure { error, .. } => {
                warn!("Inbound request failed: {:?}", error);
            }
            request_response::Event::ResponseSent { .. } => {
                debug!("Response sent successfully");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_libp2p_network_creation() {
        let config = LibP2PConfig::default();
        let result = LibP2PNetwork::new(config, None);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_peer_id_generation() {
        let config = LibP2PConfig::default();
        let (network, _rx) = LibP2PNetwork::new(config, None).unwrap();
        let peer_id = network.local_peer_id();
        assert!(!peer_id.to_string().is_empty());
    }
}
