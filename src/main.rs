mod blockdag;

use blockdag::{BlockDAG, Transaction, Block, BlockApproval};
use libp2p::{
    identity, PeerId, Multiaddr,
    kad::{record::store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent},
    gossipsub::{self, Gossipsub, GossipsubEvent, MessageAuthenticity, IdentTopic as Topic},
    noise, tcp, Transport, core::upgrade,
    relay::Relay,
    autonat, identify,
    dns::DnsConfig,
    swarm::{Swarm, SwarmEvent},
};
use futures::prelude::*;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use serde_json;
use std::env;
use rand::seq::SliceRandom;
use rand::Rng;

fn current_system_time() -> u64 {
    chrono::Utc::now().timestamp() as u64
}

// For demo: static validator set. In production, use on-chain registration.
fn validator_pubkeys() -> Vec<String> {
    vec![
        "VAL1PUBKEY".to_string(),
        "VAL2PUBKEY".to_string(),
        "VAL3PUBKEY".to_string(),
        "VAL4PUBKEY".to_string(),
    ]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // === Node identity ===
    let id_keys = identity::Keypair::generate_ed25519();
    let my_pubkey = hex::encode(id_keys.public().encode());
    let peer_id = PeerId::from(id_keys.public());
    println!("This node: pubkey = {}", my_pubkey);

    // === BlockDAG instance (thread-safe) ===
    let validators = validator_pubkeys();
    let dag = Arc::new(Mutex::new(BlockDAG::new(validators.clone())));

    // === Networking: libp2p ===
    let transport = DnsConfig::system(
        tcp::tokio::Transport::new(tcp::Config::default())
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&id_keys).unwrap())
            .multiplex(libp2p::yamux::YamuxConfig::default())
            .boxed()
    ).await?;

    let store = MemoryStore::new(peer_id.clone());
    let kad_cfg = KademliaConfig::default();
    let mut kademlia = Kademlia::with_config(peer_id.clone(), store, kad_cfg);

    // Optionally add a bootstrap peer as CLI arg
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let ma: Multiaddr = args[1].parse()?;
        let peer = ma.iter().last().and_then(|p| match p {
            libp2p::multiaddr::Protocol::P2p(hash) => PeerId::from_multihash(hash).ok(),
            _ => None
        });
        if let Some(peer_id) = peer {
            kademlia.add_address(&peer_id, ma.clone());
            println!("Bootstrap: Added peer {peer_id} @ {ma}");
        }
    }

    let mut gossipsub = Gossipsub::new(
        MessageAuthenticity::Signed(id_keys.clone()),
        gossipsub::GossipsubConfig::default()
    ).unwrap();
    let block_topic = Topic::new("blocks");
    let approval_topic = Topic::new("approvals");
    gossipsub.subscribe(&block_topic).unwrap();
    gossipsub.subscribe(&approval_topic).unwrap();

    let behaviour = libp2p::swarm::NetworkBehaviour::compose()
        .with(kademlia)
        .with(gossipsub)
        .with(Relay::new(peer_id.clone(), Default::default()))
        .with(identify::Behaviour::new(identify::Config::new("ippan/1.0".to_string(), id_keys.public())))
        .with(autonat::Behaviour::new(peer_id.clone(), Default::default()));

    let mut swarm = Swarm::with_tokio_executor(transport, behaviour, peer_id.clone());

    let listen_port = 30000 + (rand::random::<u16>() % 20000);
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port).parse()?;
    swarm.listen_on(listen_addr.clone())?;
    println!("Listening on {listen_addr}/p2p/{}", peer_id);

    // === State: track approvals, seen blocks ===
    let mut seen_blocks: HashSet<String> = HashSet::new();
    let mut seen_approvals: HashSet<String> = HashSet::new();

    // === Main event loop ===
    loop {
        futures::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(libp2p::swarm::ToSwarm::GenerateEvent(event)) => {
                        // -- Handle gossipsub events (blocks/approvals) --
                        if let Some(gossip) = event.downcast_ref::<GossipsubEvent>() {
                            match gossip {
                                GossipsubEvent::Message { message, source, .. } => {
                                    let dag = dag.clone();
                                    // Detect topic
                                    let topic = message.topic.as_str();
                                    if topic == "blocks" {
                                        if let Ok(block) = serde_json::from_slice::<Block>(&message.data) {
                                            let peer_id_str = source.as_ref().map(|p| p.to_string()).unwrap_or_default();
                                            let reported_time = block.timestamp; // block proposer reports time
                                            {
                                                let mut dag = dag.lock().unwrap();
                                                dag.update_peer_time(&peer_id_str, reported_time);
                                                if dag.validate_block(&block, 120) {
                                                    if !seen_blocks.contains(&block.hash) {
                                                        dag.add_block(block.clone());
                                                        seen_blocks.insert(block.hash.clone());
                                                        println!("[BLOCK] Received block {}", block.hash);
                                                    }
                                                } else {
                                                    println!("Rejected block (timestamp drift or invalid)");
                                                }
                                            }
                                        }
                                    }
                                    else if topic == "approvals" {
                                        if let Ok(approval) = serde_json::from_slice::<BlockApproval>(&message.data) {
                                            let peer_id_str = source.as_ref().map(|p| p.to_string()).unwrap_or_default();
                                            let reported_time = approval.reported_time;
                                            {
                                                let mut dag = dag.lock().unwrap();
                                                dag.update_peer_time(&peer_id_str, reported_time);
                                                if !seen_approvals.contains(&approval.signature) {
                                                    dag.handle_approval(approval.clone());
                                                    seen_approvals.insert(approval.signature.clone());
                                                    println!("[APPROVAL] Received for {}", approval.block_hash);
                                                }
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                        // Handle Kademlia events, etc (for diagnostics)
                        if let Some(kad) = event.downcast_ref::<KademliaEvent>() {
                            if let KademliaEvent::OutboundQueryCompleted { result, .. } = kad {
                                if let libp2p::kad::QueryResult::GetClosestPeers(Ok(ok)) = result {
                                    if !ok.peers.is_empty() {
                                        println!("Discovered peers: {:?}", ok.peers);
                                    }
                                }
                            }
                        }
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {:?}", address);
                        println!("Advertise: {address}/p2p/{}", peer_id);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("Connected to {:?}", peer_id);
                    }
                    _ => {}
                }
            },
            // --- Example block proposal (auto-mine every 60s for demo) ---
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                let dag = dag.clone();
                let my_pubkey = my_pubkey.clone();
                let block_topic = block_topic.clone();
                let mut dag = dag.lock().unwrap();

                // Only validator nodes propose blocks; check if we are in validator set
                if dag.validators.contains(&my_pubkey) {
                    // For demo: one dummy tx
                    let tx = Transaction {
                        from: my_pubkey.clone(),
                        to: "SOMEBODY".to_string(),
                        amount: rand::thread_rng().gen_range(1..100),
                        timestamp: current_system_time(),
                        hash: String::new(),
                    };
                    let parents = dag.blocks.keys().cloned().collect::<Vec<_>>();
                    let block = dag.propose_block(&my_pubkey, vec![tx], parents);
                    let block_bytes = serde_json::to_vec(&block).unwrap();
                    let _ = swarm.behaviour_mut().gossipsub.publish(&block_topic, block_bytes);
                    println!("[PROPOSED BLOCK] {}", block.hash);

                    // Automatically approve own block for demo
                    let approval = BlockApproval {
                        block_hash: block.hash.clone(),
                        validator: my_pubkey.clone(),
                        signature: format!("sig-{}", my_pubkey), // stub
                        reported_time: block.timestamp,
                    };
                    let approval_bytes = serde_json::to_vec(&approval).unwrap();
                    let _ = swarm.behaviour_mut().gossipsub.publish(&approval_topic, approval_bytes);
                    dag.handle_approval(approval);
                }
            }
        }
    }
}
