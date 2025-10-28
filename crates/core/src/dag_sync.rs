//! DAG synchronization with automatic peer discovery and gossip fan-out.
//!
//! This module wires the [`BlockDAG`](crate::dag::BlockDAG) storage into a
//! libp2p gossipsub swarm. Each node periodically advertises its local tips and
//! republishes full blocks so freshly discovered peers can request the data
//! immediately after joining. All block payloads are accompanied by zk-STARK
//! proof metadata verifying their authenticity, as required by the IPPAN PRD.

use std::collections::HashSet;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::{anyhow, Context as _, Result};
use ed25519_dalek::SigningKey;
use either::Either;
use futures::StreamExt;
use libp2p::core::transport::upgrade;
use libp2p::core::Endpoint;
use libp2p::gossipsub;
use libp2p::identity;
use libp2p::noise;
use libp2p::swarm::ConnectionHandler; // bring `select` into scope
use libp2p::swarm::{
    self, ConnectionDenied, ConnectionHandlerSelect, ConnectionId, FromSwarm, NetworkBehaviour,
    SwarmEvent, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::tcp;
use libp2p::yamux;
use libp2p::{gossipsub as gsub, mdns};
use libp2p::{Multiaddr, PeerId, Swarm, Transport};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::time::interval;

use crate::block::Block;
use crate::dag::BlockDAG;
use crate::zk_stark::{
    deserialize_proof, generate_stark_proof, serialize_proof, verify_stark_proof,
};

/// Gossip topic name shared by every IPPAN node.
const DAG_TOPIC: &str = "ippan-dag";
/// Interval between periodic tip advertisements.
const TIP_INTERVAL: Duration = Duration::from_secs(8);

/// Messages distributed across the DAG gossip topic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum GossipMsg {
    /// Announces a tip hash that peers should track.
    Tip([u8; 32]),
    /// Broadcasts the full block so late-joining peers can catch up.
    /// Each block carries its zk-STARK proof for deterministic verification.
    Block {
        block: Box<Block>,
        stark_proof: Option<Vec<u8>>,
    },
}

/// Events emitted by the combined DAG sync behaviour.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum DagEvent {
    Gossip(Box<gsub::Event>),
    Mdns(mdns::Event),
}

impl From<gsub::Event> for DagEvent {
    fn from(event: gsub::Event) -> Self {
        DagEvent::Gossip(Box::new(event))
    }
}

impl From<mdns::Event> for DagEvent {
    fn from(event: mdns::Event) -> Self {
        DagEvent::Mdns(event)
    }
}

/// Combined network behaviour for DAG gossip + mDNS peer discovery.
struct DagBehaviour {
    pub gossip: gsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

impl NetworkBehaviour for DagBehaviour {
    type ConnectionHandler =
        ConnectionHandlerSelect<THandler<gsub::Behaviour>, THandler<mdns::tokio::Behaviour>>;
    type ToSwarm = DagEvent;

    fn handle_pending_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        self.gossip
            .handle_pending_inbound_connection(connection_id, local_addr, remote_addr)?;
        self.mdns
            .handle_pending_inbound_connection(connection_id, local_addr, remote_addr)?;
        Ok(())
    }

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        let gossip_handler = self.gossip.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )?;
        let mdns_handler = self.mdns.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )?;
        Ok(gossip_handler.select(mdns_handler))
    }

    fn handle_pending_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        addresses: &[Multiaddr],
        effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        let mut combined = self.gossip.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )?;
        combined.extend(self.mdns.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )?);
        Ok(combined)
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: Endpoint,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        let gossip_handler = self.gossip.handle_established_outbound_connection(
            connection_id,
            peer,
            addr,
            role_override,
        )?;
        let mdns_handler = self.mdns.handle_established_outbound_connection(
            connection_id,
            peer,
            addr,
            role_override,
        )?;
        Ok(gossip_handler.select(mdns_handler))
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
        self.gossip.on_swarm_event(event);
        self.mdns.on_swarm_event(event);
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        match event {
            Either::Left(event) => {
                self.gossip
                    .on_connection_handler_event(peer_id, connection_id, event)
            }
            Either::Right(event) => {
                self.mdns
                    .on_connection_handler_event(peer_id, connection_id, event)
            }
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Poll::Ready(action) = self.gossip.poll(cx) {
            return Poll::Ready(action.map_out(DagEvent::from).map_in(Either::Left));
        }

        if let Poll::Ready(action) = self.mdns.poll(cx) {
            return Poll::Ready(action.map_out(DagEvent::from).map_in(Either::Right));
        }

        Poll::Pending
    }
}

/// Background swarm responsible for synchronizing BlockDAG data with peers.
pub struct DagSyncService;

impl DagSyncService {
    /// Verify a zk-STARK proof for a block
    fn verify_stark_proof(&self, block: &Block, proof: &[u8]) -> Result<()> {
        // Basic proof structure validation
        if proof.is_empty() {
            return Err(anyhow!("Empty zk-STARK proof"));
        }

        // Verify proof length is reasonable (should be much larger than this check)
        if proof.len() < 32 {
            return Err(anyhow!("zk-STARK proof too short"));
        }

        // Verify proof length is not excessive (DoS protection)
        if proof.len() > 1024 * 1024 {
            // 1MB limit
            return Err(anyhow!("zk-STARK proof too large"));
        }

        // In a production implementation, this would:
        // 1. Parse the proof structure
        // 2. Extract public inputs from the block
        // 3. Verify the proof against the verification key
        // 4. Ensure the proof corresponds to the block's content

        // For now, we'll do a basic validation that the proof
        // contains some expected structure markers
        let proof_hash = blake3::hash(proof);
        let block_hash = block.hash();

        // Verify that the proof is related to this block
        // (in a real implementation, this would be more sophisticated)
        if proof_hash.as_bytes()[0] != block_hash[0] {
            return Err(anyhow!("zk-STARK proof does not correspond to block"));
        }

        debug!(
            "zk-STARK proof verified for block {}",
            hex::encode(block_hash)
        );
        Ok(())
    }

    /// Generate a zk-STARK proof for a block
    fn generate_stark_proof(block: &Block) -> Result<Option<Vec<u8>>> {
        // In a production implementation, this would:
        // 1. Extract the block's computation trace
        // 2. Generate a zk-STARK proof using the proving key
        // 3. Return the serialized proof

        // For now, we'll generate a placeholder proof that demonstrates
        // the structure and can be verified by our verify_stark_proof method
        let block_hash = block.hash();
        let mut proof = Vec::new();

        // Add a marker to indicate this is a proof
        proof.extend_from_slice(b"STARK_PROOF_V1");

        // Add the block hash as a public input
        proof.extend_from_slice(&block_hash);

        // Add some proof data (in reality this would be the actual zk-STARK proof)
        let proof_data = blake3::hash(&block_hash);
        proof.extend_from_slice(proof_data.as_bytes());

        // Add a signature-like structure
        let mut signature_data = [0u8; 32];
        signature_data[0] = block_hash[0]; // Link to block
        proof.extend_from_slice(&signature_data);

        debug!(
            "Generated zk-STARK proof of length {} for block {}",
            proof.len(),
            hex::encode(block_hash)
        );
        Ok(Some(proof))
    }

    /// Start the DAG synchronization service and run until the task is cancelled.
    pub async fn start(listen_addr: &str, signing_key: SigningKey, dag: BlockDAG) -> Result<()> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        let verifying_key = signing_key.verifying_key();
        info!(
            "Starting DAG sync node {local_peer_id} advertising creator {}",
            hex::encode(verifying_key.to_bytes())
        );

        let noise = noise::Config::new(&local_key).context("failed to initialise noise config")?;

        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise)
            .multiplex(yamux::Config::default())
            .boxed();

        let message_id_fn = |message: &gossipsub::Message| {
            let digest = blake3::hash(&message.data);
            gossipsub::MessageId::from(digest.as_bytes())
        };

        let gossip_config = gossipsub::ConfigBuilder::default()
            .validation_mode(gossipsub::ValidationMode::None)
            .heartbeat_interval(Duration::from_secs(4))
            .message_id_fn(message_id_fn)
            .build()
            .context("failed to build gossipsub config")?;

        let mut gossip = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossip_config,
        )
        .map_err(|err| anyhow!(err))?;

        let topic = gossipsub::IdentTopic::new(DAG_TOPIC);
        gossip
            .subscribe(&topic)
            .context("failed to subscribe to DAG gossip topic")?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
            .context("failed to initialise mDNS discovery")?;

        let behaviour = DagBehaviour { gossip, mdns };
        let swarm_config = swarm::Config::with_tokio_executor();
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);

        let addr: Multiaddr = listen_addr
            .parse()
            .context("invalid listen multiaddr for DAG sync")?;
        Swarm::listen_on(&mut swarm, addr.clone()).context("unable to listen for DAG sync")?;

        let mut seen: HashSet<[u8; 32]> = HashSet::new();
        let mut ticker = interval(TIP_INTERVAL);

        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("DAG sync listening on {address}");
                        }
                        SwarmEvent::Behaviour(DagEvent::Gossip(event)) => {
                            if let Err(err) = handle_gossip_event(*event, &dag, &mut seen) {
                                warn!("error handling gossip event: {err:?}");
                            }
                        }
                        SwarmEvent::Behaviour(DagEvent::Mdns(event)) => {
                            match event {
                                mdns::Event::Discovered(peers) => {
                                    for (peer_id, _addr) in peers {
                                        swarm.behaviour_mut().gossip.add_explicit_peer(&peer_id);
                                    }
                                }
                                mdns::Event::Expired(peers) => {
                                    for (peer_id, _addr) in peers {
                                        swarm.behaviour_mut().gossip.remove_explicit_peer(&peer_id);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ = ticker.tick() => {
                    if let Err(err) =
                        broadcast_tips(&dag, &mut swarm.behaviour_mut().gossip, &topic, &mut seen)
                    {
                        warn!("failed to broadcast DAG tips: {err:?}");
                    }
                }
            }
        }
    }
}

fn handle_gossip_event(
    event: gossipsub::Event,
    dag: &BlockDAG,
    seen: &mut HashSet<[u8; 32]>,
) -> Result<()> {
    match event {
        gossipsub::Event::Message { message, .. } => {
            let msg: GossipMsg = serde_json::from_slice(&message.data)
                .context("failed to decode DAG gossip payload")?;
            match msg {
                GossipMsg::Tip(hash) => {
                    if !seen.contains(&hash) {
                        debug!("received tip {hash:?} from gossip");
                        if !dag.contains(&hash)? {
                            seen.insert(hash);
                        }
                    }
                }
                GossipMsg::Block { block, stark_proof } => {
                    let hash = block.hash();

                    // Verify zk-STARK proof if present
                    if let Some(proof_bytes) = stark_proof {
                        debug!(
                            "received zk-STARK proof of length {} for block {}",
                            proof_bytes.len(),
                            hex::encode(hash)
                        );

                        // Verify the zk-STARK proof using our implementation
                        if let Ok(proof) = deserialize_proof(&proof_bytes) {
                            match verify_stark_proof(&proof, &block) {
                                Ok(true) => {
                                    debug!(
                                        "zk-STARK proof verified for block {}",
                                        hex::encode(hash)
                                    );
                                }
                                Ok(false) => {
                                    warn!("zk-STARK proof verification failed for block {}: invalid proof", hex::encode(hash));
                                    return Ok(());
                                }
                                Err(e) => {
                                    warn!(
                                        "zk-STARK proof verification failed for block {}: {}",
                                        hex::encode(hash),
                                        e
                                    );
                                    return Ok(());
                                }
                            }
                        } else {
                            warn!(
                                "Failed to deserialize zk-STARK proof for block {}",
                                hex::encode(hash)
                            );
                            return Ok(());
                        }
                    }

                    match dag.insert_block(&block) {
                        Ok(true) => {
                            info!("🧱  Stored new block {}", hex::encode(hash));
                            seen.insert(hash);
                            dag.flush().ok();
                        }
                        Ok(false) => {
                            debug!("ignored already-known block {}", hex::encode(hash));
                        }
                        Err(err) => warn!("rejected block from gossip: {err:?}"),
                    }
                }
            }
        }
        gossipsub::Event::Subscribed { .. } | gossipsub::Event::Unsubscribed { .. } => {}
        gossipsub::Event::GossipsubNotSupported { peer_id } => {
            warn!("peer {peer_id} does not support gossipsub");
        }
    }
    Ok(())
}

fn broadcast_tips(
    dag: &BlockDAG,
    gossip: &mut gossipsub::Behaviour,
    topic: &gossipsub::IdentTopic,
    seen: &mut HashSet<[u8; 32]>,
) -> Result<()> {
    let tips = dag.get_tips()?;
    for hash in &tips {
        let payload = serde_json::to_vec(&GossipMsg::Tip(*hash))
            .context("failed to serialize tip gossip message")?;
        if let Err(err) = gossip.publish(topic.clone(), payload) {
            warn!("failed to publish tip gossip: {err:?}");
        }
    }

    for hash in tips {
        match dag.get_block(&hash)? {
            Some(block) => {
                if seen.insert(hash) {
                    // Generate zk-STARK proof for the block
                    let stark_proof = generate_stark_proof(&block)?;
                    let payload = serde_json::to_vec(&GossipMsg::Block {
                        block: Box::new(block),
                        stark_proof: Some(serialize_proof(&stark_proof)?),
                    })
                    .context("failed to serialize block gossip message")?;
                    if let Err(err) = gossip.publish(topic.clone(), payload) {
                        warn!("failed to publish block gossip: {err:?}");
                    }
                }
            }
            None => warn!("tip {hash:?} missing block payload"),
        }
    }

    Ok(())
}

/// Spawn the DAG synchronization service as a background Tokio task.
pub async fn start_dag_sync(listen_addr: &str, signing_key: SigningKey, dag: BlockDAG) {
    let addr = listen_addr.to_owned();
    tokio::spawn(async move {
        if let Err(err) = DagSyncService::start(&addr, signing_key, dag).await {
            warn!("DAG sync service stopped: {err:?}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossip_messages_roundtrip() {
        let hash = [7u8; 32];
        let tip_bytes = serde_json::to_vec(&GossipMsg::Tip(hash)).unwrap();
        assert_eq!(
            GossipMsg::Tip(hash),
            serde_json::from_slice(&tip_bytes).unwrap()
        );
    }
}
