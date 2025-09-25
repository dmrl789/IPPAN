use anyhow::Result;
use clap::{Arg, Command};
use config::Config;
use ippan_consensus::{PoAConfig, PoAConsensus, Validator};
use ippan_p2p::{HttpP2PNetwork, P2PConfig};
use ippan_rpc::{start_server, AppState, ConsensusHandle};
use ippan_storage::SledStorage;
use ippan_types::{ippan_time_init, ippan_time_now, HashTimer, IppanTimeMicros};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Application configuration
#[derive(Debug, Clone)]
struct AppConfig {
    // Node identity
    node_id: String,
    validator_id: [u8; 32],

    // Network
    rpc_host: String,
    rpc_port: u16,
    p2p_host: String,
    p2p_port: u16,

    // Storage
    data_dir: String,
    db_path: String,

    // Consensus
    slot_duration_ms: u64,
    max_transactions_per_block: usize,
    block_reward: u64,

    // P2P
    bootstrap_nodes: Vec<String>,
    max_peers: usize,
    peer_discovery_interval_secs: u64,
    peer_announce_interval_secs: u64,
    p2p_public_host: Option<String>,
    p2p_enable_upnp: bool,
    p2p_external_ip_services: Vec<String>,

    // Logging
    log_level: String,
    log_format: String,

    // Development
    dev_mode: bool,
}

impl AppConfig {
    fn load() -> Result<Self> {
        // Load from environment variables with defaults
        let config = Config::builder()
            .add_source(config::Environment::with_prefix("IPPAN"))
            .build()?;

        // Parse validator ID from hex string
        let validator_id_str = config.get_string("VALIDATOR_ID").unwrap_or_else(|_| {
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
        });

        let validator_id = if validator_id_str.len() == 64 {
            let mut id = [0u8; 32];
            hex::decode_to_slice(&validator_id_str, &mut id)?;
            id
        } else {
            // Generate a deterministic ID from the string
            let mut id = [0u8; 32];
            let hash_bytes = validator_id_str.as_bytes();
            for (i, &byte) in hash_bytes.iter().enumerate() {
                if i < 32 {
                    id[i] = byte;
                }
            }
            id
        };

        let external_ip_services: Vec<String> = config
            .get_string("P2P_EXTERNAL_IP_SERVICES")
            .unwrap_or_else(|_| "https://api.ipify.org,https://ifconfig.me/ip".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            node_id: config
                .get_string("NODE_ID")
                .unwrap_or_else(|_| "ippan_node".to_string()),
            validator_id,
            rpc_host: config
                .get_string("RPC_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            rpc_port: config
                .get_string("RPC_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            p2p_host: config
                .get_string("P2P_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            p2p_port: config
                .get_string("P2P_PORT")
                .unwrap_or_else(|_| "9000".to_string())
                .parse()?,
            data_dir: config
                .get_string("DATA_DIR")
                .unwrap_or_else(|_| "./data".to_string()),
            db_path: config
                .get_string("DB_PATH")
                .unwrap_or_else(|_| "./data/db".to_string()),
            slot_duration_ms: config
                .get_string("SLOT_DURATION_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,
            max_transactions_per_block: config
                .get_string("MAX_TRANSACTIONS_PER_BLOCK")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,
            block_reward: config
                .get_string("BLOCK_REWARD")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
            bootstrap_nodes: config
                .get_string("BOOTSTRAP_NODES")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            max_peers: config
                .get_string("MAX_PEERS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()?,
            peer_discovery_interval_secs: config
                .get_string("PEER_DISCOVERY_INTERVAL_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            peer_announce_interval_secs: config
                .get_string("PEER_ANNOUNCE_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
            p2p_public_host: config
                .get_string("P2P_PUBLIC_HOST")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string()),
            p2p_enable_upnp: config
                .get_string("P2P_ENABLE_UPNP")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            p2p_external_ip_services: if external_ip_services.is_empty() {
                vec![
                    "https://api.ipify.org".to_string(),
                    "https://ifconfig.me/ip".to_string(),
                ]
            } else {
                external_ip_services
            },
            log_level: config
                .get_string("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            log_format: config
                .get_string("LOG_FORMAT")
                .unwrap_or_else(|_| "pretty".to_string()),
            dev_mode: config
                .get_string("DEV_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("ippan-node")
        .version(env!("CARGO_PKG_VERSION"))
        .about("IPPAN Blockchain Node")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path"),
        )
        .arg(
            Arg::new("data-dir")
                .short('d')
                .long("data-dir")
                .value_name("DIR")
                .help("Data directory"),
        )
        .arg(Arg::new("dev").long("dev").help("Run in development mode"))
        .get_matches();

    // Load configuration
    let mut config = AppConfig::load()?;

    // Override with command line arguments
    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.data_dir = data_dir.clone();
        config.db_path = format!("{}/db", data_dir);
    }

    if matches.get_flag("dev") {
        config.dev_mode = true;
        config.log_level = "debug".to_string();
        config.log_format = "pretty".to_string();
    }

    // Initialize logging
    init_logging(&config)?;

    // Initialize IPPAN Time service
    ippan_time_init();
    info!("IPPAN Time service initialized");

    info!("Starting IPPAN node: {}", config.node_id);
    info!("Validator ID: {}", hex::encode(config.validator_id));
    info!("Data directory: {}", config.data_dir);
    info!("Development mode: {}", config.dev_mode);

    // Create data directory
    std::fs::create_dir_all(&config.data_dir)?;

    // Initialize storage
    let storage = Arc::new(SledStorage::new(&config.db_path)?);
    storage.initialize()?;
    info!("Storage initialized at {}", config.db_path);

    // Initialize consensus
    let consensus_config = PoAConfig {
        slot_duration_ms: config.slot_duration_ms,
        validators: vec![Validator {
            id: config.validator_id,
            address: config.validator_id,
            stake: 1000000,
            is_active: true,
        }],
        max_transactions_per_block: config.max_transactions_per_block,
        block_reward: config.block_reward,
    };

    let consensus_instance =
        PoAConsensus::new(consensus_config, storage.clone(), config.validator_id);
    let tx_sender = consensus_instance.get_tx_sender();
    let mempool = consensus_instance.mempool();
    let consensus = Arc::new(Mutex::new(consensus_instance));
    {
        let mut consensus_guard = consensus.lock().await;
        consensus_guard.start().await?;
    }
    info!("Consensus engine started");

    // Initialize P2P network
    let p2p_config = P2PConfig {
        listen_address: format!("http://{}:{}", config.p2p_host, config.p2p_port),
        bootstrap_peers: config.bootstrap_nodes.clone(),
        max_peers: config.max_peers,
        peer_discovery_interval: Duration::from_secs(config.peer_discovery_interval_secs),
        message_timeout: Duration::from_secs(10),
        retry_attempts: 3,
        public_host: config.p2p_public_host.clone(),
        enable_upnp: config.p2p_enable_upnp,
        external_ip_services: config.p2p_external_ip_services.clone(),
        peer_announce_interval: Duration::from_secs(config.peer_announce_interval_secs),
    };

    let local_p2p_address = format!("http://{}:{}", config.p2p_host, config.p2p_port);
    let mut p2p_network = HttpP2PNetwork::new(p2p_config, local_p2p_address)?;
    p2p_network.start().await?;
    info!(
        "HTTP P2P network started on {}:{}",
        config.p2p_host, config.p2p_port
    );

    // Get peer ID before moving p2p_network
    let peer_id = p2p_network.get_local_peer_id();

    // Create transaction channel for consensus
    let consensus_handle = ConsensusHandle::new(consensus.clone(), tx_sender.clone(), mempool);

    // Initialize RPC server
    let peer_count = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();

    let p2p_network_arc = Arc::new(p2p_network);
    let p2p_network_for_shutdown = p2p_network_arc.clone();
    let consensus_for_shutdown = consensus.clone();
    peer_count.store(p2p_network_arc.get_peer_count(), Ordering::Relaxed);
    let app_state = AppState {
        storage: storage.clone(),
        start_time,
        peer_count: peer_count.clone(),
        p2p_network: Some(p2p_network_arc.clone()),
        tx_sender: Some(tx_sender.clone()),
        consensus: Some(consensus_handle.clone()),
        node_id: config.node_id.clone(),
    };

    let rpc_addr = format!("{}:{}", config.rpc_host, config.rpc_port);
    let rpc_addr_clone = rpc_addr.clone();
    info!("Starting RPC server on {}", rpc_addr);

    // Start RPC server in background
    let rpc_handle = tokio::spawn(async move {
        if let Err(e) = start_server(app_state, &rpc_addr_clone).await {
            error!("RPC server error: {}", e);
        }
    });

    // Create a boot HashTimer to uniquely identify this startup
    let current_time = IppanTimeMicros(ippan_time_now());
    let domain = "boot";
    let payload = b"node_startup";
    let nonce = ippan_types::random_nonce();
    let node_id = config.node_id.as_bytes();

    let boot_hashtimer = HashTimer::derive(
        domain,
        current_time,
        domain.as_bytes(),
        payload,
        &nonce,
        node_id,
    );

    info!("Boot HashTimer: {}", boot_hashtimer.to_hex());
    info!("Current IPPAN Time: {} microseconds", current_time.0);

    let consensus_state = {
        let consensus_guard = consensus.lock().await;
        consensus_guard.get_state()
    };
    info!(
        "Consensus state => slot: {}, latest height: {}, proposer: {:?}",
        consensus_state.current_slot,
        consensus_state.latest_block_height,
        consensus_state.current_proposer.map(hex::encode)
    );

    // Update peer count periodically
    let peer_count_updater = {
        let peer_count = peer_count.clone();
        let network = p2p_network_arc.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let count = network.get_peer_count();
                peer_count.store(count, Ordering::Relaxed);
            }
        })
    };

    info!("IPPAN node is ready and running");
    info!("RPC API available at: http://{}", rpc_addr);
    info!(
        "P2P network listening on: {}:{}",
        config.p2p_host, config.p2p_port
    );
    info!("Node ID: {}", peer_id);

    // Keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down IPPAN node");

    // Stop components gracefully
    {
        let mut consensus_guard = consensus_for_shutdown.lock().await;
        consensus_guard.stop().await?;
    }
    p2p_network_for_shutdown.stop().await?;
    rpc_handle.abort();
    peer_count_updater.abort();

    // Flush storage
    storage.flush()?;

    info!("IPPAN node shutdown complete");
    Ok(())
}

fn init_logging(config: &AppConfig) -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    if config.log_format == "json" {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }

    Ok(())
}
