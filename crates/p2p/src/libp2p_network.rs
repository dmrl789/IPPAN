//! libp2p-based networking stack for the IPPAN node.
//!
//! This module complements the HTTP fallback (`HttpP2PNetwork`) with a fully
//! fledged libp2p swarm that supports gossip, peer discovery, relays, and
//! NAT hole-punching. It exposes a small command API so the rest of the
//! workspace can drive the swarm without importing libp2p directly.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use blake3;
use futures::StreamExt;
use libp2p::core::transport::OrTransport;
use libp2p::core::upgrade;
use libp2p::dcutr;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::identity;
use libp2p::kad;
use libp2p::multiaddr::Protocol;
use libp2p::noise;
use libp2p::ping;
use libp2p::relay;
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use libp2p::tcp;
use libp2p::yamux;
use libp2p::{mdns, Multiaddr, PeerId, Transport};
use parking_lot::{Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, trace, warn};

/// Default gossip topics propagated across the libp2p fabric.
pub const DEFAULT_GOSSIP_TOPICS: &[&str] =
    &["ippan/blocks", "ippan/transactions", "ippan/peer-info"];

/// Configuration for the libp2p network.
#[derive(Debug, Clone)]
pub struct Libp2pConfig {
    /// Addresses to listen on. If empty, defaults to `/ip4/0.0.0.0/tcp/9000`.
    pub listen_addresses: Vec<Multiaddr>,
    /// Bootstrap peers dialed on startup.
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Dedicated relay servers for NAT traversal.
    pub relay_servers: Vec<Multiaddr>,
    /// Additional gossip topics to subscribe to (defaults included).
    pub gossip_topics: Vec<String>,
    /// Whether to enable mDNS discovery on local networks.
    pub enable_mdns: bool,
    /// Whether to enable relay and DCUtR hole punching.
    pub enable_relay: bool,
    /// Optional deterministic identity keypair.
    pub identity_keypair: Option<identity::Keypair>,
    /// Protocol version string.
    pub protocol_version: String,
    /// Agent version identifier.
    pub agent_version: String,
    /// Bootstrap retry interval (for cold-start recovery).
    pub bootstrap_retry_interval: Duration,
    /// Maximum bootstrap retry attempts (0 = infinite).
    pub bootstrap_max_retries: usize,
}

impl Default for Libp2pConfig {
    fn default() -> Self {
        let default_listen =
            Multiaddr::from_str("/ip4/0.0.0.0/tcp/9000").expect("valid default multiaddr");
        Self {
            listen_addresses: vec![default_listen],
            bootstrap_peers: Vec::new(),
            relay_servers: Vec::new(),
            gossip_topics: DEFAULT_GOSSIP_TOPICS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            enable_mdns: true,
            enable_relay: true,
            identity_keypair: None,
            protocol_version: "/ippan/1.0.0".to_string(),
            agent_version: format!("ippan-p2p/{}", env!("CARGO_PKG_VERSION")),
            bootstrap_retry_interval: Duration::from_secs(30),
            bootstrap_max_retries: 0, // infinite retries by default
        }
    }
}

/// Commands used to control the background swarm task.
#[derive(Debug)]
pub enum Libp2pCommand {
    Publish {
        topic: String,
        data: Vec<u8>,
    },
    Dial {
        address: Multiaddr,
    },
    AddExplicitPeer {
        peer_id: PeerId,
        address: Option<Multiaddr>,
    },
    RemoveExplicitPeer {
        peer_id: PeerId,
        address: Option<Multiaddr>,
    },
    Shutdown,
}

/// Events produced by the libp2p network.
#[derive(Debug, Clone)]
pub enum Libp2pEvent {
    Gossip {
        peer: PeerId,
        topic: String,
        data: Vec<u8>,
    },
    PeerDiscovered {
        peers: Vec<(PeerId, Vec<Multiaddr>)>,
    },
    PeerConnected {
        peer: PeerId,
    },
    PeerDisconnected {
        peer: PeerId,
    },
    NewListenAddr {
        address: Multiaddr,
    },
    RelayReservationAccepted {
        relay: PeerId,
    },
    HolePunchSucceeded {
        peer: PeerId,
    },
    HolePunchFailed {
        peer: PeerId,
        error: String,
    },
}

/// Combined behaviour of libp2p protocols.
///
/// This struct integrates all libp2p protocols used by IPPAN:
/// - **Kademlia DHT**: Peer discovery and routing table maintenance. Currently used for
///   node discovery only; IPNDHT-specific features (handle lookup, file/hash listing)
///   are planned but not yet implemented.
/// - **GossipSub**: Efficient pub/sub for blocks and transactions.
/// - **mDNS**: Zero-config local network peer discovery.
/// - **Relay + DCUtR**: NAT traversal for connectivity behind firewalls.
/// - **Identify**: Automatic peer information exchange.
/// - **Ping**: Connection health monitoring.
///
/// See `docs/ipndht/ipndht_hardening_plan.md` for future DHT enhancements.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
struct ComposedBehaviour {
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    /// Kademlia DHT for peer routing and future IPNDHT features (handle lookup, content addressing).
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    mdns: Toggle<mdns::tokio::Behaviour>,
    relay: Toggle<relay::client::Behaviour>,
    dcutr: Toggle<dcutr::Behaviour>,
}

impl ComposedBehaviour {
    fn new(
        local_key: &identity::Keypair,
        peer_id: PeerId,
        config: &Libp2pConfig,
        relay_behaviour: Option<relay::client::Behaviour>,
    ) -> Result<Self> {
        let message_id_fn = |message: &gossipsub::Message| {
            let digest = blake3::hash(&message.data);
            gossipsub::MessageId::from(digest.as_bytes())
        };

        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .validation_mode(gossipsub::ValidationMode::Strict)
            .heartbeat_interval(Duration::from_secs(4))
            .build()
            .map_err(|e| anyhow!("failed to build gossipsub config: {e}"))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossip_config,
        )
        .map_err(|e| anyhow!("failed to construct gossipsub: {e}"))?;

        let identify = identify::Behaviour::new(
            identify::Config::new(config.protocol_version.clone(), local_key.public())
                .with_agent_version(config.agent_version.clone()),
        );

        let ping = ping::Behaviour::default();

        let store = kad::store::MemoryStore::new(peer_id);
        let mut kad_cfg = kad::Config::default();
        kad_cfg.set_query_timeout(Duration::from_secs(5));
        let kademlia = kad::Behaviour::with_config(peer_id, store, kad_cfg);

        let mdns_behaviour = Toggle::from(if config.enable_mdns {
            Some(mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                peer_id,
            )?)
        } else {
            None
        });

        let relay_behaviour = Toggle::from(relay_behaviour);
        let dcutr_behaviour = Toggle::from(if config.enable_relay {
            Some(dcutr::Behaviour::new(peer_id))
        } else {
            None
        });

        Ok(Self {
            gossipsub,
            identify,
            ping,
            kademlia,
            mdns: mdns_behaviour,
            relay: relay_behaviour,
            dcutr: dcutr_behaviour,
        })
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum ComposedEvent {
    Gossipsub(Box<gossipsub::Event>),
    Identify(identify::Event),
    Ping(ping::Event),
    Kademlia(kad::Event),
    Mdns(mdns::Event),
    Relay(relay::client::Event),
    Dcutr(dcutr::Event),
}

impl From<gossipsub::Event> for ComposedEvent {
    fn from(value: gossipsub::Event) -> Self {
        Self::Gossipsub(Box::new(value))
    }
}
impl From<identify::Event> for ComposedEvent {
    fn from(v: identify::Event) -> Self {
        Self::Identify(v)
    }
}
impl From<ping::Event> for ComposedEvent {
    fn from(v: ping::Event) -> Self {
        Self::Ping(v)
    }
}
impl From<kad::Event> for ComposedEvent {
    fn from(v: kad::Event) -> Self {
        Self::Kademlia(v)
    }
}
impl From<mdns::Event> for ComposedEvent {
    fn from(v: mdns::Event) -> Self {
        Self::Mdns(v)
    }
}
impl From<relay::client::Event> for ComposedEvent {
    fn from(v: relay::client::Event) -> Self {
        Self::Relay(v)
    }
}
impl From<dcutr::Event> for ComposedEvent {
    fn from(v: dcutr::Event) -> Self {
        Self::Dcutr(v)
    }
}

/// Main libp2p network controller.
pub struct Libp2pNetwork {
    peer_id: PeerId,
    command_tx: mpsc::UnboundedSender<Libp2pCommand>,
    events_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Libp2pEvent>>>>,
    listen_addresses: Arc<RwLock<HashSet<Multiaddr>>>,
    _task: JoinHandle<()>,
}

impl Libp2pNetwork {
    /// Initialise libp2p and spawn background swarm task.
    pub fn new(config: Libp2pConfig) -> Result<Self> {
        let keypair = config
            .identity_keypair
            .clone()
            .unwrap_or_else(identity::Keypair::generate_ed25519);
        let peer_id = PeerId::from(keypair.public());
        info!("Initialising libp2p peer {}", peer_id);

        let (transport, relay_behaviour) = if config.enable_relay {
            let (relay_transport, relay_behaviour) = relay::client::new(peer_id);
            let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
            let transport = OrTransport::new(relay_transport, tcp_transport)
                .upgrade(upgrade::Version::V1)
                .authenticate(noise::Config::new(&keypair)?)
                .multiplex(yamux::Config::default())
                .boxed();
            (transport, Some(relay_behaviour))
        } else {
            let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
            let transport = tcp_transport
                .upgrade(upgrade::Version::V1)
                .authenticate(noise::Config::new(&keypair)?)
                .multiplex(yamux::Config::default())
                .boxed();
            (transport, None)
        };

        let behaviour = ComposedBehaviour::new(&keypair, peer_id, &config, relay_behaviour)?;
        let swarm_config = libp2p::swarm::Config::with_tokio_executor();
        let mut swarm = Swarm::new(transport, behaviour, peer_id, swarm_config);

        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<Libp2pCommand>();
        let (event_tx, events_rx) = mpsc::unbounded_channel::<Libp2pEvent>();
        let events_rx = Arc::new(Mutex::new(Some(events_rx)));
        let listen_addresses = Arc::new(RwLock::new(HashSet::<Multiaddr>::new()));

        let mut topic_map: HashMap<String, gossipsub::IdentTopic> = HashMap::new();
        let mut combined = HashSet::new();
        for t in DEFAULT_GOSSIP_TOPICS {
            combined.insert(t.to_string());
        }
        for t in config.gossip_topics.iter() {
            combined.insert(t.clone());
        }
        for topic_name in combined {
            let topic = gossipsub::IdentTopic::new(&topic_name);
            if let Err(err) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                warn!("Failed to subscribe to topic {topic_name}: {err}");
            }
            topic_map.insert(topic_name, topic);
        }

        for addr in if config.listen_addresses.is_empty() {
            vec![Multiaddr::from_str("/ip4/0.0.0.0/tcp/9000")?]
        } else {
            config.listen_addresses.clone()
        } {
            if Swarm::listen_on(&mut swarm, addr.clone()).is_ok() {
                listen_addresses.write().insert(addr);
            }
        }

        for address in config
            .bootstrap_peers
            .iter()
            .chain(config.relay_servers.iter())
        {
            if let Err(err) = swarm.dial(address.clone()) {
                warn!("Failed to dial bootstrap {}: {}", address, err);
            }
        }

        let relay_peer_ids: HashSet<PeerId> = config
            .relay_servers
            .iter()
            .filter_map(|addr| extract_peer_id(addr).ok().flatten())
            .collect();

        // Bootstrap retry logic for cold-start recovery
        let bootstrap_peers_for_retry = config
            .bootstrap_peers
            .iter()
            .chain(config.relay_servers.iter())
            .cloned()
            .collect::<Vec<_>>();
        let bootstrap_retry_interval = config.bootstrap_retry_interval;
        let bootstrap_max_retries = config.bootstrap_max_retries;

        let listen_task = listen_addresses.clone();
        let events_tx_task = event_tx.clone();
        let task = tokio::spawn(async move {
            let mut bootstrap_ticker = tokio::time::interval(bootstrap_retry_interval);
            bootstrap_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut retry_count = 0;

            loop {
                tokio::select! {
                    _ = bootstrap_ticker.tick() => {
                        // Retry bootstrap if we have no peers and retries are not exhausted
                        if !bootstrap_peers_for_retry.is_empty() {
                            let connected_peers = swarm.connected_peers().count();
                            if connected_peers == 0 {
                                if bootstrap_max_retries == 0 || retry_count < bootstrap_max_retries {
                                    retry_count += 1;
                                    debug!("Bootstrap retry attempt {} (connected peers: {})", retry_count, connected_peers);
                                    for addr in &bootstrap_peers_for_retry {
                                        if let Err(err) = swarm.dial(addr.clone()) {
                                            debug!("Bootstrap retry dial failed for {}: {}", addr, err);
                                        }
                                    }
                                } else {
                                    debug!("Bootstrap retry exhausted (max: {})", bootstrap_max_retries);
                                }
                            } else {
                                // Reset retry count if we have peers
                                if retry_count > 0 {
                                    debug!("Bootstrap successful, resetting retry count");
                                    retry_count = 0;
                                }
                            }
                        }
                    }
                    swarm_event = swarm.select_next_some() => {
                        handle_swarm_event(swarm_event, &mut swarm, &events_tx_task, &listen_task, &relay_peer_ids);
                    }
                    cmd = command_rx.recv() => {
                        match cmd {
                            Some(Libp2pCommand::Shutdown) => {
                                debug!("Shutting down libp2p swarm");
                                break;
                            }
                            Some(other) => {
                                if let Err(e) = handle_command(other, &mut swarm, &mut topic_map) {
                                    warn!("Failed to handle libp2p command: {e}");
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
            info!("libp2p swarm terminated");
        });

        Ok(Self {
            peer_id,
            command_tx,
            events_rx,
            listen_addresses,
            _task: task,
        })
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn listen_addresses(&self) -> Vec<Multiaddr> {
        self.listen_addresses.read().iter().cloned().collect()
    }

    pub fn take_event_receiver(&self) -> Option<mpsc::UnboundedReceiver<Libp2pEvent>> {
        self.events_rx.lock().take()
    }

    pub fn publish<T: Into<String>>(&self, topic: T, data: Vec<u8>) -> Result<()> {
        self.command_tx
            .send(Libp2pCommand::Publish {
                topic: topic.into(),
                data,
            })
            .map_err(|_| anyhow!("libp2p command channel closed"))
    }

    pub fn dial(&self, address: Multiaddr) -> Result<()> {
        self.command_tx
            .send(Libp2pCommand::Dial { address })
            .map_err(|_| anyhow!("libp2p command channel closed"))
    }

    pub fn add_explicit_peer(&self, peer_id: PeerId, address: Option<Multiaddr>) -> Result<()> {
        self.command_tx
            .send(Libp2pCommand::AddExplicitPeer { peer_id, address })
            .map_err(|_| anyhow!("libp2p command channel closed"))
    }

    pub fn remove_explicit_peer(&self, peer_id: PeerId, address: Option<Multiaddr>) -> Result<()> {
        self.command_tx
            .send(Libp2pCommand::RemoveExplicitPeer { peer_id, address })
            .map_err(|_| anyhow!("libp2p command channel closed"))
    }

    pub fn shutdown(&self) {
        let _ = self.command_tx.send(Libp2pCommand::Shutdown);
    }
}

impl Drop for Libp2pNetwork {
    fn drop(&mut self) {
        let _ = self.command_tx.send(Libp2pCommand::Shutdown);
    }
}

fn handle_swarm_event(
    event: SwarmEvent<ComposedEvent>,
    swarm: &mut Swarm<ComposedBehaviour>,
    event_tx: &mpsc::UnboundedSender<Libp2pEvent>,
    listen_addresses: &Arc<RwLock<HashSet<Multiaddr>>>,
    relay_peers: &HashSet<PeerId>,
) {
    match event {
        SwarmEvent::Behaviour(ComposedEvent::Gossipsub(event)) => {
            if let gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            } = *event
            {
                let _ = event_tx.send(Libp2pEvent::Gossip {
                    peer: propagation_source,
                    topic: message.topic.to_string(),
                    data: message.data.clone(),
                });
            }
        }
        SwarmEvent::Behaviour(ComposedEvent::Identify(event)) => {
            trace!("Received identify event: {:?}", event);
        }
        SwarmEvent::Behaviour(ComposedEvent::Ping(event)) => {
            trace!("Received ping event: {:?}", event);
        }
        SwarmEvent::Behaviour(ComposedEvent::Kademlia(event)) => {
            trace!("Received Kademlia event: {:?}", event);
        }
        SwarmEvent::Behaviour(ComposedEvent::Mdns(event)) => match event {
            mdns::Event::Discovered(discovered) => {
                let mut aggregate: HashMap<PeerId, Vec<Multiaddr>> = HashMap::new();
                for (peer, addr) in discovered {
                    if peer == *swarm.local_peer_id() {
                        continue;
                    }
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer, addr.clone());
                    if swarm.dial(addr.clone()).is_ok() {
                        aggregate.entry(peer).or_default().push(addr);
                    }
                }
                if !aggregate.is_empty() {
                    let peers = aggregate.into_iter().collect();
                    let _ = event_tx.send(Libp2pEvent::PeerDiscovered { peers });
                }
            }
            mdns::Event::Expired(expired) => {
                for (peer, addr) in expired {
                    swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                    swarm.behaviour_mut().kademlia.remove_address(&peer, &addr);
                }
            }
        },
        SwarmEvent::Behaviour(ComposedEvent::Relay(
            relay::client::Event::ReservationReqAccepted { relay_peer_id, .. },
        )) => {
            let _ = event_tx.send(Libp2pEvent::RelayReservationAccepted {
                relay: relay_peer_id,
            });
        }
        SwarmEvent::Behaviour(ComposedEvent::Relay(event)) => {
            trace!("Ignoring relay client event: {:?}", event);
        }
        SwarmEvent::Behaviour(ComposedEvent::Dcutr(event)) => match event.result {
            Ok(_) => {
                let _ = event_tx.send(Libp2pEvent::HolePunchSucceeded {
                    peer: event.remote_peer_id,
                });
            }
            Err(err) => {
                let _ = event_tx.send(Libp2pEvent::HolePunchFailed {
                    peer: event.remote_peer_id,
                    error: err.to_string(),
                });
            }
        },
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            if relay_peers.contains(&peer_id) {
                debug!("Connected to relay {}", peer_id);
            }
            let _ = event_tx.send(Libp2pEvent::PeerConnected { peer: peer_id });
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            let _ = event_tx.send(Libp2pEvent::PeerDisconnected { peer: peer_id });
        }
        SwarmEvent::NewListenAddr { address, .. } => {
            listen_addresses.write().insert(address.clone());
            let _ = event_tx.send(Libp2pEvent::NewListenAddr { address });
        }
        _ => {}
    }
}

fn handle_command(
    command: Libp2pCommand,
    swarm: &mut Swarm<ComposedBehaviour>,
    topic_map: &mut HashMap<String, gossipsub::IdentTopic>,
) -> Result<()> {
    match command {
        Libp2pCommand::Publish { topic, data } => {
            let entry = topic_map.entry(topic.clone()).or_insert_with(|| {
                let topic_id = gossipsub::IdentTopic::new(&topic);
                let _ = swarm.behaviour_mut().gossipsub.subscribe(&topic_id);
                topic_id
            });
            let _ = swarm.behaviour_mut().gossipsub.publish(entry.clone(), data);
        }
        Libp2pCommand::Dial { address } => {
            let _ = swarm.dial(address.clone());
        }
        Libp2pCommand::AddExplicitPeer { peer_id, address } => {
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            if let Some(addr) = address {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr.clone());
                let _ = swarm.dial(addr);
            }
        }
        Libp2pCommand::RemoveExplicitPeer { peer_id, address } => {
            swarm
                .behaviour_mut()
                .gossipsub
                .remove_explicit_peer(&peer_id);
            if let Some(addr) = address {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .remove_address(&peer_id, &addr);
            } else {
                swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
            }
        }
        Libp2pCommand::Shutdown => {}
    }
    Ok(())
}

fn extract_peer_id(addr: &Multiaddr) -> Result<Option<PeerId>> {
    for protocol in addr.iter() {
        if let Protocol::P2p(peer_id) = protocol {
            return Ok(Some(peer_id));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_has_listen_address() {
        let config = Libp2pConfig::default();
        assert!(!config.listen_addresses.is_empty());
    }

    #[tokio::test]
    async fn network_initialises() {
        let config = Libp2pConfig {
            listen_addresses: vec![Multiaddr::from_str("/ip4/127.0.0.1/tcp/0").unwrap()],
            bootstrap_peers: Vec::new(),
            relay_servers: Vec::new(),
            gossip_topics: vec!["ippan/test".to_string()],
            enable_mdns: false,
            enable_relay: false,
            identity_keypair: None,
            protocol_version: "/ippan/test".to_string(),
            agent_version: "ippan-test/0.0.1".to_string(),
            bootstrap_retry_interval: Duration::from_secs(30),
            bootstrap_max_retries: 0,
        };

        let network = Libp2pNetwork::new(config).expect("expected network to initialise");
        assert!(!network.listen_addresses().is_empty());
        network.shutdown();
    }
}
