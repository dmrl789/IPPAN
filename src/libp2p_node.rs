use libp2p::{
    identity, PeerId, Multiaddr,
    kad::{record::store::MemoryStore, Kademlia, KademliaEvent, KademliaConfig},
    swarm::{Swarm, SwarmEvent},
    gossipsub::{self, Gossipsub, GossipsubEvent, MessageAuthenticity, IdentTopic as Topic},
    noise, tcp, Transport, core::upgrade,
    relay::Relay,
    autonat, identify,
    dns::DnsConfig,
};
use std::error::Error;
use std::sync::mpsc::{Sender, Receiver};
use futures::prelude::*;

pub enum Libp2pEvent {
    Block(Vec<u8>),
    Transaction(Vec<u8>),
}

pub struct Libp2pHandle {
    pub sender: Sender<Libp2pEvent>,
    pub receiver: Receiver<Libp2pEvent>,
    pub block_topic: Topic,
    pub tx_topic: Topic,
}

pub async fn run_libp2p_node(
    listen_port: u16,
    bootstrap_nodes: Vec<String>,
    event_sender: Sender<Libp2pEvent>,
) -> Result<(), Box<dyn Error>> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("libp2p PeerId: {}", peer_id);

    let transport = DnsConfig::system(
        tcp::tokio::Transport::new(tcp::Config::default())
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseAuthenticated::xx(&id_keys).unwrap())
        .multiplex(libp2p::yamux::YamuxConfig::default())
        .boxed()
    ).await?;

    let store = MemoryStore::new(peer_id.clone());
    let mut kad_cfg = KademliaConfig::default();
    kad_cfg.set_query_timeout(std::time::Duration::from_secs(10));
    let mut kademlia = Kademlia::with_config(peer_id.clone(), store, kad_cfg);

    for addr in bootstrap_nodes {
        if let Ok(ma) = addr.parse::<Multiaddr>() {
            if let Some(peer_id) = ma.iter().last().and_then(|p| match p {
                libp2p::multiaddr::Protocol::P2p(hash) => PeerId::from_multihash(hash).ok(),
                _ => None
            }) {
                kademlia.add_address(&peer_id, ma.clone());
                println!("Bootstrap: Added peer {peer_id} @ {ma}");
            }
        }
    }

    let mut gossipsub = Gossipsub::new(
        MessageAuthenticity::Signed(id_keys.clone()),
        gossipsub::GossipsubConfig::default()
    ).unwrap();
    let block_topic = Topic::new("blocks");
    let tx_topic = Topic::new("txs");
    gossipsub.subscribe(&block_topic).unwrap();
    gossipsub.subscribe(&tx_topic).unwrap();

    let behaviour = libp2p::swarm::NetworkBehaviour::compose()
        .with(kademlia)
        .with(gossipsub)
        .with(Relay::new(peer_id.clone(), Default::default()))
        .with(identify::Behaviour::new(identify::Config::new("ippan/1.0".to_string(), id_keys.public())))
        .with(autonat::Behaviour::new(peer_id.clone(), Default::default()));

    let mut swarm = Swarm::with_tokio_executor(transport, behaviour, peer_id.clone());

    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port).parse()?;
    swarm.listen_on(listen_addr)?;

    println!("libp2p node running. Listening for peers, blocks, txs...");
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(libp2p::swarm::ToSwarm::GenerateEvent(event)) => {
                // Gossipsub events (incoming blocks/txs)
                if let Some(gossip) = event.downcast_ref::<GossipsubEvent>() {
                    match gossip {
                        GossipsubEvent::Message { message, .. } => {
                            let topic = message.topic.as_str();
                            let data = message.data.clone();
                            if topic == "blocks" {
                                let _ = event_sender.send(Libp2pEvent::Block(data));
                            } else if topic == "txs" {
                                let _ = event_sender.send(Libp2pEvent::Transaction(data));
                            }
                        },
                        _ => {}
                    }
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {:?}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("Connected to {:?}", peer_id);
            }
            _ => {}
        }
    }
}
