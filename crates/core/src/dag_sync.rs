//! DAG synchronization with automatic peer discovery and gossip fan-out.
//!
//! This module wires the [`BlockDAG`](crate::dag::BlockDAG) storage into a
//! libp2p gossipsub swarm. Each node periodically advertises its local tips and
//! republishes full blocks so freshly discovered peers can request the data
//! immediately after joining.

use std::collections::HashSet;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::{anyhow, Context as AnyhowContext, Result};
use ed25519_dalek::SigningKey;
use futures::StreamExt;
use libp2p::core::transport::upgrade;
use libp2p::gossipsub;
use libp2p::identity;
use libp2p::noise;
use libp2p::swarm::behaviour::FromSwarm;
use libp2p::swarm::derive_prelude::Either;
use libp2p::swarm::{
    self, ConnectionDenied, ConnectionHandler, ConnectionHandlerSelect, ConnectionId,
    NetworkBehaviour, SwarmEvent, THandler, THandlerInEvent, THandlerOutEvent, ToSwarm,
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

/// Gossip topic name shared by every IPPAN node.
const DAG_TOPIC: &str = "ippan-dag";
/// Interval between periodic tip advertisements.
const TIP_INTERVAL: Duration = Duration::from_secs(8);

/// Messages distributed across the DAG gossip topic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GossipMsg {
    /// Announces a tip hash that peers should track.
    Tip([u8; 32]),
    /// Broadcasts the full block so late-joining peers can catch up.
    Block(Block),
}

/// Combined network behaviour for DAG gossip + mDNS peer discovery.
struct DagBehaviour {
    pub gossip: gsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
enum DagEvent {
    Gossip(gossipsub::Event),
    Mdns(mdns::Event),
}

type GossipHandler = <gsub::Behaviour as NetworkBehaviour>::ConnectionHandler;
type MdnsHandler = <mdns::tokio::Behaviour as NetworkBehaviour>::ConnectionHandler;

impl NetworkBehaviour for DagBehaviour {
    type ConnectionHandler = ConnectionHandlerSelect<GossipHandler, MdnsHandler>;
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
    ) -> Result<THandler<Self>, ConnectionDenied> {
        let gossip = self
            .gossip
            .handle_established_inbound_connection(connection_id, peer, local_addr, remote_addr)?;
        let mdns = self
            .mdns
            .handle_established_inbound_connection(connection_id, peer, local_addr, remote_addr)?;

        Ok(gossip.select(mdns))
    }

    fn handle_pending_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        addresses: &[Multiaddr],
        effective_role: libp2p::core::Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        let mut addrs = self.gossip.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )?;
        addrs.extend(self.mdns.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )?);
        Ok(addrs)
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: libp2p::core::Endpoint,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        let gossip = self
            .gossip
            .handle_established_outbound_connection(connection_id, peer, addr, role_override)?;
        let mdns = self
            .mdns
            .handle_established_outbound_connection(connection_id, peer, addr, role_override)?;
        Ok(gossip.select(mdns))
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
        match event {
            FromSwarm::ConnectionEstablished(ev) => {
                self.gossip
                    .on_swarm_event(FromSwarm::ConnectionEstablished(ev));
                self.mdns
                    .on_swarm_event(FromSwarm::ConnectionEstablished(ev));
            }
            FromSwarm::ConnectionClosed(ev) => {
                self.gossip.on_swarm_event(FromSwarm::ConnectionClosed(ev));
                self.mdns.on_swarm_event(FromSwarm::ConnectionClosed(ev));
            }
            FromSwarm::AddressChange(ev) => {
                self.gossip.on_swarm_event(FromSwarm::AddressChange(ev));
                self.mdns.on_swarm_event(FromSwarm::AddressChange(ev));
            }
            FromSwarm::DialFailure(ev) => {
                self.gossip.on_swarm_event(FromSwarm::DialFailure(ev));
                self.mdns.on_swarm_event(FromSwarm::DialFailure(ev));
            }
            FromSwarm::ListenFailure(ev) => {
                self.gossip
                    .on_swarm_event(FromSwarm::ListenFailure(ev));
                self.mdns
                    .on_swarm_event(FromSwarm::ListenFailure(ev));
            }
            FromSwarm::NewListener(ev) => {
                self.gossip.on_swarm_event(FromSwarm::NewListener(ev));
                self.mdns.on_swarm_event(FromSwarm::NewListener(ev));
            }
            FromSwarm::NewListenAddr(ev) => {
                self.gossip.on_swarm_event(FromSwarm::NewListenAddr(ev));
                self.mdns.on_swarm_event(FromSwarm::NewListenAddr(ev));
            }
            FromSwarm::ExpiredListenAddr(ev) => {
                self.gossip
                    .on_swarm_event(FromSwarm::ExpiredListenAddr(ev));
                self.mdns
                    .on_swarm_event(FromSwarm::ExpiredListenAddr(ev));
            }
            FromSwarm::ListenerError(ev) => {
                self.gossip.on_swarm_event(FromSwarm::ListenerError(ev));
                self.mdns.on_swarm_event(FromSwarm::ListenerError(ev));
            }
            FromSwarm::ListenerClosed(ev) => {
                self.gossip.on_swarm_event(FromSwarm::ListenerClosed(ev));
                self.mdns.on_swarm_event(FromSwarm::ListenerClosed(ev));
            }
            FromSwarm::NewExternalAddrCandidate(addr) => {
                self.gossip
                    .on_swarm_event(FromSwarm::NewExternalAddrCandidate(addr));
                self.mdns
                    .on_swarm_event(FromSwarm::NewExternalAddrCandidate(addr));
            }
            FromSwarm::ExternalAddrConfirmed(addr) => {
                self.gossip
                    .on_swarm_event(FromSwarm::ExternalAddrConfirmed(addr));
                self.mdns
                    .on_swarm_event(FromSwarm::ExternalAddrConfirmed(addr));
            }
            FromSwarm::ExternalAddrExpired(addr) => {
                self.gossip
                    .on_swarm_event(FromSwarm::ExternalAddrExpired(addr));
                self.mdns
                    .on_swarm_event(FromSwarm::ExternalAddrExpired(addr));
            }
            FromSwarm::NewExternalAddrOfPeer(ev) => {
                self.gossip
                    .on_swarm_event(FromSwarm::NewExternalAddrOfPeer(ev));
                self.mdns
                    .on_swarm_event(FromSwarm::NewExternalAddrOfPeer(ev));
            }
            _ => {}
        }
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        match event {
            Either::Left(event) => self
                .gossip
                .on_connection_handler_event(peer_id, connection_id, event),
            Either::Right(event) => self
                .mdns
                .on_connection_handler_event(peer_id, connection_id, event),
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Poll::Ready(event) = self.gossip.poll(cx) {
            return Poll::Ready(
                event
                    .map_in(|ev| Either::Left(ev))
                    .map_out(DagEvent::Gossip),
            );
        }

        if let Poll::Ready(event) = self.mdns.poll(cx) {
            return Poll::Ready(
                event
                    .map_in(|ev| Either::Right(ev))
                    .map_out(DagEvent::Mdns),
            );
        }

        Poll::Pending
    }
}

/// Background swarm responsible for synchronizing BlockDAG data with peers.
pub struct DagSyncService;

impl DagSyncService {
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
        .map_err(|err| anyhow!("failed to create gossipsub behaviour: {err}"))?;
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
                            if let Err(err) = handle_gossip_event(event, &dag, &mut seen) {
                                warn!("error handling gossip event: {err:?}");
                            }
                        }
                        SwarmEvent::Behaviour(DagEvent::Mdns(event)) => {
                            handle_mdns_event(swarm.behaviour_mut(), event);
                        }
                        _ => {}
                    }
                }
                _ = ticker.tick() => {
                    let behaviour = swarm.behaviour_mut();
                    if let Err(err) = broadcast_tips(&dag, &mut behaviour.gossip, &topic, &mut seen) {
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
                            // keep it in the seen set to avoid repeated requests until block arrives
                            seen.insert(hash);
                        }
                    }
                }
                GossipMsg::Block(block) => {
                    let hash = block.hash();
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

fn handle_mdns_event(behaviour: &mut DagBehaviour, event: mdns::Event) {
    match event {
        mdns::Event::Discovered(peers) => {
            for (peer, _addr) in peers {
                behaviour.gossip.add_explicit_peer(&peer);
            }
        }
        mdns::Event::Expired(peers) => {
            for (peer, _addr) in peers {
                if !behaviour
                    .mdns
                    .discovered_nodes()
                    .any(|p| p == &peer)
                {
                    behaviour.gossip.remove_explicit_peer(&peer);
                }
            }
        }
    }
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
                    let payload = serde_json::to_vec(&GossipMsg::Block(block))
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
