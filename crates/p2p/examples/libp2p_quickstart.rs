use anyhow::Result;
use ippan_p2p::{Libp2pConfig, Libp2pEvent, Libp2pNetwork};
use libp2p::Multiaddr;
use tokio::signal;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut config = Libp2pConfig::default();
    config.listen_addresses = vec!["/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?];
    config.agent_version = format!("ippan-libp2p-demo/{}", env!("CARGO_PKG_VERSION"));
    config.gossip_topics.push("ippan/demo".into());

    let network = Libp2pNetwork::new(config)?;
    info!("Local peer id: {}", network.peer_id());

    let events_rx = match network.take_event_receiver() {
        Some(events) => events,
        None => {
            warn!("Libp2p event receiver was already taken; shutting down");
            network.shutdown();
            return Ok(());
        }
    };

    tokio::select! {
        _ = async move {
            let mut events = events_rx;
            while let Some(event) = events.recv().await {
                log_event(event);
            }
        } => {}
        _ = signal::ctrl_c() => {
            info!("Ctrl+C received; stopping libp2p demo");
        }
    }

    network.shutdown();
    Ok(())
}

fn log_event(event: Libp2pEvent) {
    match event {
        Libp2pEvent::Gossip { peer, topic, data } => {
            info!(peer = %peer, topic = %topic, size = data.len(), "Received gossip message");
        }
        Libp2pEvent::PeerDiscovered { peers } => {
            for (peer, addresses) in peers {
                if addresses.is_empty() {
                    info!(peer = %peer, "Discovered peer with no advertised addresses");
                } else {
                    for address in addresses {
                        info!(peer = %peer, address = %address, "Discovered peer address");
                    }
                }
            }
        }
        Libp2pEvent::PeerConnected { peer } => {
            info!(peer = %peer, "Peer connected");
        }
        Libp2pEvent::PeerDisconnected { peer } => {
            info!(peer = %peer, "Peer disconnected");
        }
        Libp2pEvent::NewListenAddr { address } => {
            info!(address = %address, "Swarm is now listening");
        }
        Libp2pEvent::RelayReservationAccepted { relay } => {
            info!(relay = %relay, "Relay reservation accepted");
        }
        Libp2pEvent::HolePunchSucceeded { peer } => {
            info!(peer = %peer, "Hole punch succeeded");
        }
        Libp2pEvent::HolePunchFailed { peer, error } => {
            warn!(peer = %peer, %error, "Hole punch failed");
        }
    }
}
