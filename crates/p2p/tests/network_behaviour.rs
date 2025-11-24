use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ippan_p2p::{DhtConfig, HttpP2PNetwork, NetworkEvent, NetworkMessage, P2PConfig, PeerInfo};
use parking_lot::RwLock;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};

#[derive(Clone)]
struct MockPeerState {
    peers: Arc<RwLock<Vec<String>>>,
}

struct MockPeerServer {
    address: String,
    shutdown: Option<oneshot::Sender<()>>,
}

impl MockPeerServer {
    async fn start(initial_peers: Vec<String>) -> Self {
        let state = MockPeerState {
            peers: Arc::new(RwLock::new(initial_peers)),
        };

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock peer listener");
        let addr = listener.local_addr().expect("listener addr lookup");

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let app = Router::new()
            .route("/p2p/peers", get(get_peers))
            .route("/p2p/peer-info", post(accept_json))
            .with_state(state.clone());

        tokio::spawn(async move {
            let server = axum::serve(listener, app);
            let graceful = server.with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
            let _ = graceful.await;
        });

        Self {
            address: format!("http://{}", addr),
            shutdown: Some(shutdown_tx),
        }
    }

    fn address(&self) -> &str {
        &self.address
    }

    async fn shutdown(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

impl Drop for MockPeerServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn get_peers(State(state): State<MockPeerState>) -> Json<Vec<String>> {
    Json(state.peers.read().clone())
}

async fn accept_json(Json(_body): Json<serde_json::Value>) -> StatusCode {
    StatusCode::OK
}

fn next_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("allocate ephemeral port")
        .local_addr()
        .expect("extract ephemeral addr")
        .port()
}

fn test_config(listen_port: u16) -> P2PConfig {
    let listen_address = format!("http://127.0.0.1:{listen_port}");

    P2PConfig {
        listen_address: listen_address.clone(),
        peer_discovery_interval: Duration::from_millis(50),
        message_timeout: Duration::from_millis(100),
        retry_attempts: 1,
        dht: DhtConfig {
            bootstrap_peers: Vec::new(),
            public_host: Some(listen_address),
            enable_upnp: false,
            external_ip_services: Vec::new(),
            announce_interval: Duration::from_secs(60),
        },
        ..P2PConfig::default()
    }
}

fn metadata_by_address(metadata: Vec<PeerInfo>) -> HashMap<String, PeerInfo> {
    metadata
        .into_iter()
        .map(|info| (info.address.clone(), info))
        .collect()
}

async fn wait_for_peer(
    network: &HttpP2PNetwork,
    peer: &str,
    timeout_duration: Duration,
) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout_duration {
        if network.get_peers().iter().any(|p| p == peer) {
            return Ok(());
        }
        sleep(Duration::from_millis(25)).await;
    }
    Err(format!("timed out waiting for peer {peer}"))
}

async fn wait_for_discovery_event(
    events: &mut UnboundedReceiver<NetworkEvent>,
    peer: &str,
    timeout_duration: Duration,
) -> Result<(), String> {
    let discovery = async {
        while let Some(event) = events.recv().await {
            if let NetworkEvent::PeerDiscovery { peers, .. } = event {
                if peers.iter().any(|p| p == peer) {
                    return Ok(());
                }
            }
        }
        Err("event channel closed before discovery".to_string())
    };

    match timeout(timeout_duration, discovery).await {
        Ok(result) => result,
        Err(_) => Err(format!(
            "no discovery event for {peer} within {:?}",
            timeout_duration
        )),
    }
}

#[tokio::test]
async fn node_discovery_loop_finds_new_peer() {
    let discovered_peer = format!("http://127.0.0.1:{}", next_port());
    let mut server = MockPeerServer::start(vec![discovered_peer.clone()]).await;

    let mut config = test_config(next_port());
    config.dht.bootstrap_peers = vec![server.address().to_string()];
    let local_address = config.listen_address.clone();
    let mut network = HttpP2PNetwork::new(config, local_address).expect("network creation");

    network.start().await.expect("network start");
    let mut events = network
        .take_incoming_events()
        .expect("event receiver available");

    wait_for_discovery_event(&mut events, &discovered_peer, Duration::from_secs(2))
        .await
        .expect("discovery event received");
    wait_for_peer(&network, &discovered_peer, Duration::from_secs(2))
        .await
        .expect("peer added to topology");

    network.stop().await.expect("network stop");
    server.shutdown().await;
}

#[tokio::test]
async fn distributed_metadata_converges_across_nodes() {
    let port_a = next_port();
    let port_b = next_port();
    let addr_a = format!("http://127.0.0.1:{port_a}");
    let addr_b = format!("http://127.0.0.1:{port_b}");

    let config_a = test_config(port_a);
    let config_b = test_config(port_b);
    let network_a =
        HttpP2PNetwork::new(config_a, addr_a.clone()).expect("network A creation succeeded");
    let network_b =
        HttpP2PNetwork::new(config_b, addr_b.clone()).expect("network B creation succeeded");

    let time_primary = 1_234_567;
    network_a
        .process_incoming_message(
            &addr_b,
            NetworkMessage::PeerInfo {
                peer_id: "node-b".to_string(),
                addresses: vec![addr_b.clone()],
                time_us: Some(time_primary),
            },
        )
        .await
        .expect("A processes B info");
    network_b
        .process_incoming_message(
            &addr_a,
            NetworkMessage::PeerInfo {
                peer_id: "node-a".to_string(),
                addresses: vec![addr_a.clone()],
                time_us: Some(time_primary),
            },
        )
        .await
        .expect("B processes A info");

    let addr_c = format!("http://127.0.0.1:{}", next_port());
    let time_c = 7_654_321;
    let node_c_id = "node-c".to_string();
    network_a
        .process_incoming_message(
            &addr_b,
            NetworkMessage::PeerDiscovery {
                peers: vec![addr_c.clone()],
            },
        )
        .await
        .expect("A receives discovery for C");
    network_b
        .process_incoming_message(
            &addr_a,
            NetworkMessage::PeerDiscovery {
                peers: vec![addr_c.clone()],
            },
        )
        .await
        .expect("B receives discovery for C");

    for network in [&network_a, &network_b] {
        network
            .process_incoming_message(
                &addr_c,
                NetworkMessage::PeerInfo {
                    peer_id: node_c_id.clone(),
                    addresses: vec![addr_c.clone()],
                    time_us: Some(time_c),
                },
            )
            .await
            .expect("each node processes C metadata");
    }

    let meta_a = metadata_by_address(network_a.get_peer_metadata());
    let meta_b = metadata_by_address(network_b.get_peer_metadata());

    let info_a_b = meta_a.get(&addr_b).expect("A has B metadata");
    assert_eq!(info_a_b.id, "node-b");
    assert!(
        info_a_b.last_seen >= time_primary,
        "expected last_seen >= {time_primary}, got {}",
        info_a_b.last_seen
    );
    assert!(info_a_b.is_connected);

    let info_b_a = meta_b.get(&addr_a).expect("B has A metadata");
    assert_eq!(info_b_a.id, "node-a");
    assert!(
        info_b_a.last_seen >= time_primary,
        "expected last_seen >= {time_primary}, got {}",
        info_b_a.last_seen
    );
    assert!(info_b_a.is_connected);

    let info_a_c = meta_a.get(&addr_c).expect("A tracks C metadata");
    let info_b_c = meta_b.get(&addr_c).expect("B tracks C metadata");
    assert_eq!(info_a_c.id, node_c_id);
    assert_eq!(info_b_c.id, info_a_c.id);
    assert!(
        info_a_c.last_seen >= time_c,
        "expected last_seen >= {time_c}, got {}",
        info_a_c.last_seen
    );
    assert!(
        info_b_c.last_seen >= time_c,
        "expected last_seen >= {time_c}, got {}",
        info_b_c.last_seen
    );
    assert!(info_a_c.is_connected);
    assert!(info_b_c.is_connected);
}

#[tokio::test]
async fn peer_reconnection_restores_metadata() {
    let peer_addr = format!("http://127.0.0.1:{}", next_port());
    let config = test_config(next_port());
    let local_address = config.listen_address.clone();
    let network = HttpP2PNetwork::new(config, local_address).expect("network creation");

    network
        .add_peer(peer_addr.clone())
        .await
        .expect("initial peer add");
    assert!(
        network.get_peers().contains(&peer_addr),
        "peer present after initial add"
    );

    network.remove_peer(&peer_addr);
    assert!(
        !network.get_peers().contains(&peer_addr),
        "peer removed from topology"
    );
    assert!(
        !metadata_by_address(network.get_peer_metadata()).contains_key(&peer_addr),
        "metadata cleared after remove"
    );

    let reconnection_time = 42_000;
    network
        .process_incoming_message(
            &peer_addr,
            NetworkMessage::PeerInfo {
                peer_id: "rejoining-peer".to_string(),
                addresses: vec![peer_addr.clone()],
                time_us: Some(reconnection_time),
            },
        )
        .await
        .expect("process reconnection announcement");

    assert!(
        network.get_peers().contains(&peer_addr),
        "peer re-added after reconnection"
    );

    let metadata = metadata_by_address(network.get_peer_metadata());
    let info = metadata
        .get(&peer_addr)
        .expect("metadata restored for reconnecting peer");
    assert_eq!(info.id, "rejoining-peer");
    assert_eq!(info.last_seen, reconnection_time);
    assert!(info.is_connected);
}
