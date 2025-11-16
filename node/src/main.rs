use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use config::Config;
use hex::encode as hex_encode;
use ippan_consensus::{
    DLCConfig, DLCIntegratedConsensus, PoAConfig, PoAConsensus, Validator, VALIDATOR_BOND_AMOUNT,
};
use ippan_consensus_dlc::{DlcConfig as AiDlcConfig, DlcConsensus};
use ippan_files::{dht::StubFileDhtService, FileDhtService, FileStorage, MemoryFileStorage};
use ippan_l1_handle_anchors::L1HandleAnchorStorage;
use ippan_l2_handle_registry::L2HandleRegistry;
use ippan_mempool::Mempool;
use ippan_p2p::{
    HttpP2PNetwork, IpnDhtService, Libp2pConfig, Libp2pFileDhtService, Libp2pNetwork, Multiaddr,
    NetworkEvent, P2PConfig,
};
use ippan_rpc::server::ConsensusHandle;
use ippan_rpc::{start_server, AiStatusHandle, AppState, L2Config};
use ippan_security::{SecurityConfig as RpcSecurityConfig, SecurityManager as RpcSecurityManager};
use ippan_storage::{SledStorage, Storage};
use ippan_types::{
    ippan_time_init, ippan_time_now, Block, HashTimer, IppanTimeMicros, Transaction,
};
use metrics::describe_gauge;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileDhtMode {
    Stub,
    Libp2p,
}

impl FileDhtMode {
    fn from_env(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "libp2p" => FileDhtMode::Libp2p,
            _ => FileDhtMode::Stub,
        }
    }
}

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
    consensus_mode: String,
    slot_duration_ms: u64,
    max_transactions_per_block: usize,
    block_reward: u64,
    finalization_interval_ms: u64,

    // DLC Configuration
    enable_dlc: bool,
    temporal_finality_ms: u64,
    shadow_verifier_count: usize,
    min_reputation_score: i32,
    enable_dgbdt_fairness: bool,
    enable_shadow_verifiers: bool,
    require_validator_bond: bool,

    // L2 interoperability
    l2_max_commit_size: usize,
    l2_min_epoch_gap_ms: u64,
    l2_challenge_window_ms: u64,
    l2_da_mode: String,
    l2_max_l2_count: usize,

    // P2P
    bootstrap_nodes: Vec<String>,
    max_peers: usize,
    peer_discovery_interval_secs: u64,
    peer_announce_interval_secs: u64,
    p2p_public_host: Option<String>,
    p2p_enable_upnp: bool,
    p2p_external_ip_services: Vec<String>,

    // File DHT
    file_dht_mode: FileDhtMode,
    file_dht_listen_multiaddrs: Vec<String>,
    file_dht_bootstrap_multiaddrs: Vec<String>,

    // Unified UI
    unified_ui_dist_dir: Option<PathBuf>,

    // Security
    enable_security: bool,

    // Observability
    prometheus_enabled: bool,

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
            for (i, &byte) in hash_bytes.iter().enumerate().take(32) {
                id[i] = byte;
            }
            id
        };

        let mut external_ip_services: Vec<String> = config
            .get_string("P2P_EXTERNAL_IP_SERVICES")
            .unwrap_or_else(|_| "https://api.ipify.org,https://ifconfig.me/ip".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if external_ip_services.is_empty() {
            external_ip_services.push("https://api.ipify.org".to_string());
            external_ip_services.push("https://ifconfig.me/ip".to_string());
        }

        let file_dht_mode = FileDhtMode::from_env(
            &config
                .get_string("FILE_DHT_MODE")
                .unwrap_or_else(|_| "stub".to_string()),
        );

        let mut file_dht_listen_multiaddrs: Vec<String> = config
            .get_string("FILE_DHT_LIBP2P_LISTEN")
            .unwrap_or_else(|_| "/ip4/0.0.0.0/tcp/9100".to_string())
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if file_dht_listen_multiaddrs.is_empty() {
            file_dht_listen_multiaddrs.push("/ip4/0.0.0.0/tcp/9100".to_string());
        }

        let file_dht_bootstrap_multiaddrs: Vec<String> = config
            .get_string("FILE_DHT_LIBP2P_BOOTSTRAP")
            .unwrap_or_else(|_| String::new())
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        let unified_ui_dist_dir = config
            .get_string("UNIFIED_UI_DIST_DIR")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                let default_path = PathBuf::from("./apps/unified-ui/dist");
                if default_path.exists() {
                    Some(default_path)
                } else {
                    None
                }
            });

        Ok(Self {
            node_id: config
                .get_string("NODE_ID")
                .unwrap_or_else(|_| "ippan_node".to_string()),
            validator_id,
            consensus_mode: config
                .get_string("CONSENSUS_MODE")
                .unwrap_or_else(|_| "POA".to_string()),
            enable_dlc: config
                .get_string("ENABLE_DLC")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
            temporal_finality_ms: config
                .get_string("TEMPORAL_FINALITY_MS")
                .unwrap_or_else(|_| "250".to_string())
                .parse()?,
            shadow_verifier_count: config
                .get_string("SHADOW_VERIFIER_COUNT")
                .unwrap_or_else(|_| "3".to_string())
                .parse()?,
            min_reputation_score: config
                .get_string("MIN_REPUTATION_SCORE")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()?,
            enable_dgbdt_fairness: config.get_bool("ENABLE_DGBDT_FAIRNESS").unwrap_or(true),
            enable_shadow_verifiers: config.get_bool("ENABLE_SHADOW_VERIFIERS").unwrap_or(true),
            require_validator_bond: config.get_bool("REQUIRE_VALIDATOR_BOND").unwrap_or(true),
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
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
            max_transactions_per_block: config
                .get_string("MAX_TRANSACTIONS_PER_BLOCK")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,
            block_reward: config
                .get_string("BLOCK_REWARD")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
            finalization_interval_ms: config
                .get_string("FINALIZATION_INTERVAL_MS")
                .unwrap_or_else(|_| "200".to_string())
                .parse()?,
            l2_max_commit_size: config
                .get_string("L2_MAX_COMMIT_SIZE")
                .unwrap_or_else(|_| "16384".to_string())
                .parse()?,
            l2_min_epoch_gap_ms: config
                .get_string("L2_MIN_EPOCH_GAP_MS")
                .unwrap_or_else(|_| "250".to_string())
                .parse()?,
            l2_challenge_window_ms: config
                .get_string("L2_CHALLENGE_WINDOW_MS")
                .unwrap_or_else(|_| "60000".to_string())
                .parse()?,
            l2_da_mode: config
                .get_string("L2_DA_MODE")
                .unwrap_or_else(|_| "external".to_string()),
            l2_max_l2_count: config
                .get_string("L2_MAX_L2_COUNT")
                .unwrap_or_else(|_| "100".to_string())
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
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            p2p_enable_upnp: config.get_bool("P2P_ENABLE_UPNP").unwrap_or(false),
            p2p_external_ip_services: external_ip_services,
            file_dht_mode,
            file_dht_listen_multiaddrs,
            file_dht_bootstrap_multiaddrs,
            unified_ui_dist_dir,
            enable_security: config.get_bool("ENABLE_SECURITY").unwrap_or(true),
            prometheus_enabled: config.get_bool("PROMETHEUS_ENABLED").unwrap_or(true),
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

fn parse_multiaddrs(values: &[String], label: &str) -> Vec<Multiaddr> {
    values
        .iter()
        .filter_map(|value| {
            if value.is_empty() {
                return None;
            }
            match Multiaddr::from_str(value) {
                Ok(addr) => Some(addr),
                Err(err) => {
                    warn!("Invalid {} multiaddr {}: {}", label, value, err);
                    None
                }
            }
        })
        .collect()
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
        .arg(
            Arg::new("dev")
                .long("dev")
                .action(ArgAction::SetTrue)
                .help("Run in development mode"),
        )
        .get_matches();

    // Load configuration
    let mut config = AppConfig::load()?;

    // Override with command line arguments
    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.data_dir = data_dir.clone();
        config.db_path = format!("{data_dir}/db");
    }

    if matches.get_flag("dev") {
        config.dev_mode = true;
        config.log_level = "debug".to_string();
        config.log_format = "pretty".to_string();
    }

    // Initialize logging
    init_logging(&config)?;

    // Initialize metrics exporter
    let prometheus_handle = init_metrics(&config);

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

    let handle_registry = Arc::new(L2HandleRegistry::new());
    let handle_anchors = Arc::new(L1HandleAnchorStorage::new());

    // Initialize consensus
    let consensus_config = PoAConfig {
        slot_duration_ms: config.slot_duration_ms,
        validators: vec![Validator {
            id: config.validator_id,
            address: config.validator_id,
            stake: 1_000_000,
            is_active: true,
        }],
        max_transactions_per_block: config.max_transactions_per_block,
        block_reward: config.block_reward,
        finalization_interval_ms: config.finalization_interval_ms,
        enable_ai_reputation: false,
        enable_fee_caps: true,
        enable_dag_fair_emission: true,
    };

    // Initialize consensus based on mode
    let (tx_sender, mempool, consensus);
    let mut ai_status_handle: Option<AiStatusHandle> = None;

    if config.consensus_mode.to_uppercase() == "DLC" || config.enable_dlc {
        info!("Starting DLC consensus mode");

        // Create base PoA consensus
        let poa_instance = PoAConsensus::with_handle_services(
            consensus_config.clone(),
            storage.clone(),
            config.validator_id,
            handle_registry.clone(),
            handle_anchors.clone(),
        );

        // Create DLC configuration
        let dlc_config = DLCConfig {
            temporal_finality_ms: config.temporal_finality_ms,
            hashtimer_precision_us: 1,
            shadow_verifier_count: config.shadow_verifier_count,
            min_reputation_score: config.min_reputation_score,
            max_transactions_per_block: config.max_transactions_per_block,
            enable_dgbdt_fairness: config.enable_dgbdt_fairness,
            enable_shadow_verifiers: config.enable_shadow_verifiers,
            require_validator_bond: config.require_validator_bond,
            dag_config: Default::default(),
        };

        // Create integrated DLC consensus
        let mut dlc_integrated =
            DLCIntegratedConsensus::new(poa_instance, dlc_config, config.validator_id);

        tx_sender = dlc_integrated.poa.get_tx_sender();
        mempool = dlc_integrated.poa.mempool();

        // Add validator bond if required
        if config.require_validator_bond {
            info!(
                "Adding validator bond of {} micro-IPN (10 IPN)",
                VALIDATOR_BOND_AMOUNT
            );
            if let Err(e) =
                dlc_integrated.add_validator_bond(config.validator_id, VALIDATOR_BOND_AMOUNT)
            {
                warn!("Failed to add validator bond: {}", e);
            }
        }

        // Start DLC consensus
        dlc_integrated.start().await?;
        consensus = Arc::new(Mutex::new(dlc_integrated.poa));
        ai_status_handle = build_dlc_ai_status_handle();

        info!("DLC consensus engine started");
        info!("  - Temporal finality: {}ms", config.temporal_finality_ms);
        info!("  - Shadow verifiers: {}", config.shadow_verifier_count);
        info!("  - D-GBDT fairness: {}", config.enable_dgbdt_fairness);
        info!("  - Validator bonding: {}", config.require_validator_bond);
    } else {
        info!("Starting PoA consensus mode");
        let consensus_instance = PoAConsensus::with_handle_services(
            consensus_config,
            storage.clone(),
            config.validator_id,
            handle_registry.clone(),
            handle_anchors.clone(),
        );
        tx_sender = consensus_instance.get_tx_sender();
        mempool = consensus_instance.mempool();
        consensus = Arc::new(Mutex::new(consensus_instance));
        {
            let mut consensus_guard = consensus.lock().await;
            consensus_guard.start().await?;
        }
        info!("PoA consensus engine started");
    }

    // Initialize P2P network
    let p2p_host = &config.p2p_host;
    let p2p_port = config.p2p_port;
    let listen_address = format!("http://{p2p_host}:{p2p_port}");
    let p2p_config = P2PConfig {
        listen_address: listen_address.clone(),
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

    let mut p2p_network = HttpP2PNetwork::new(p2p_config, listen_address.clone())?;
    p2p_network.start().await?;
    info!(
        "HTTP P2P network started on {}:{}",
        config.p2p_host, config.p2p_port
    );

    let mut incoming_events = p2p_network.take_incoming_events();

    // Get peer ID before moving p2p_network
    let peer_id = p2p_network.get_local_peer_id();

    // Wire consensus handle for RPC
    let consensus_handle =
        ConsensusHandle::new(consensus.clone(), tx_sender.clone(), mempool.clone());

    // Initialize RPC server
    let peer_count = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();

    let p2p_network_arc = Arc::new(p2p_network);
    let p2p_network_for_shutdown = p2p_network_arc.clone();
    let consensus_for_shutdown = consensus.clone();
    peer_count.store(p2p_network_arc.get_peer_count(), Ordering::Relaxed);

    let consensus_for_events = consensus_handle.clone();
    if let Some(mut events) = incoming_events.take() {
        let network_for_events = p2p_network_arc.clone();
        let storage_for_events: Arc<dyn Storage + Send + Sync> = storage.clone();
        let mempool_for_events = mempool.clone();

        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let Err(err) = handle_p2p_event(
                    event,
                    storage_for_events.clone(),
                    mempool_for_events.clone(),
                    Some(consensus_for_events.clone()),
                    network_for_events.clone(),
                )
                .await
                {
                    warn!("Failed to handle inbound P2P event: {}", err);
                }
            }
        });
    }

    let l2_config = L2Config {
        max_commit_size: config.l2_max_commit_size,
        min_epoch_gap_ms: config.l2_min_epoch_gap_ms,
        challenge_window_ms: config.l2_challenge_window_ms,
        da_mode: config.l2_da_mode.clone(),
        max_l2_count: config.l2_max_l2_count,
    };

    let security = if config.enable_security {
        match RpcSecurityManager::new(RpcSecurityConfig::default()) {
            Ok(manager) => {
                info!("RPC security manager enabled");
                Some(Arc::new(manager))
            }
            Err(err) => {
                warn!("Failed to initialise RPC security manager: {}", err);
                None
            }
        }
    } else {
        info!("RPC security manager disabled by configuration");
        None
    };

    let file_storage: Arc<dyn FileStorage> = Arc::new(MemoryFileStorage::new());
    let file_dht: Arc<dyn FileDhtService> = match config.file_dht_mode {
        FileDhtMode::Stub => {
            info!("File DHT mode set to stub");
            Arc::new(StubFileDhtService::new())
        }
        FileDhtMode::Libp2p => {
            info!("File DHT mode set to libp2p");
            let listen_multiaddrs =
                parse_multiaddrs(&config.file_dht_listen_multiaddrs, "FILE_DHT_LIBP2P_LISTEN");
            let bootstrap_multiaddrs = parse_multiaddrs(
                &config.file_dht_bootstrap_multiaddrs,
                "FILE_DHT_LIBP2P_BOOTSTRAP",
            );
            let mut libp2p_config = Libp2pConfig::default();
            if !listen_multiaddrs.is_empty() {
                libp2p_config.listen_addresses = listen_multiaddrs;
            }
            libp2p_config.bootstrap_peers = bootstrap_multiaddrs;
            if !libp2p_config
                .gossip_topics
                .iter()
                .any(|topic| topic == "ippan/files")
            {
                libp2p_config.gossip_topics.push("ippan/files".to_string());
            }
            match Libp2pNetwork::new(libp2p_config) {
                Ok(network) => {
                    let network = Arc::new(network);
                    let addresses = network.listen_addresses();
                    info!("File DHT libp2p listening on {:?}", addresses);
                    let ipn_service = Arc::new(IpnDhtService::new(Some(network)));
                    Arc::new(Libp2pFileDhtService::new(ipn_service))
                }
                Err(err) => {
                    warn!(
                        "Failed to initialise libp2p File DHT (fallback to stub): {}",
                        err
                    );
                    Arc::new(StubFileDhtService::new())
                }
            }
        }
    };

    let app_state = AppState {
        storage: storage.clone(),
        start_time,
        peer_count: peer_count.clone(),
        p2p_network: Some(p2p_network_arc.clone()),
        tx_sender: Some(tx_sender.clone()),
        node_id: config.node_id.clone(),
        consensus: Some(consensus_handle.clone()),
        ai_status: ai_status_handle,
        l2_config,
        mempool: mempool.clone(),
        unified_ui_dist: config.unified_ui_dist_dir.clone(),
        req_count: Arc::new(AtomicUsize::new(0)),
        security,
        metrics: prometheus_handle.clone(),
        file_storage: Some(file_storage),
        file_dht: Some(file_dht),
        dev_mode: config.dev_mode,
        handle_registry: handle_registry.clone(),
        handle_anchors: handle_anchors.clone(),
    };

    let rpc_host = &config.rpc_host;
    let rpc_port = config.rpc_port;
    let rpc_addr = format!("{rpc_host}:{rpc_port}");
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

async fn handle_p2p_event(
    event: NetworkEvent,
    storage: Arc<dyn Storage + Send + Sync>,
    mempool: Arc<Mempool>,
    consensus: Option<ConsensusHandle>,
    network: Arc<HttpP2PNetwork>,
) -> Result<()> {
    match event {
        NetworkEvent::Block { block, .. } | NetworkEvent::BlockResponse { block, .. } => {
            persist_block_from_peer(&storage, &mempool, &block)?;
        }
        NetworkEvent::Transaction { transaction, .. } => {
            persist_transaction_from_peer(&storage, &mempool, consensus, &transaction)?;
        }
        NetworkEvent::PeerDiscovery { peers, .. } => {
            for peer in peers {
                if let Err(err) = network.add_peer(peer.clone()).await {
                    warn!("Failed to add discovered peer {}: {}", peer, err);
                }
            }
        }
        NetworkEvent::PeerInfo { .. } | NetworkEvent::BlockRequest { .. } => {
            // Already handled internally by HttpP2PNetwork; nothing extra to do here.
        }
    }

    Ok(())
}

fn persist_block_from_peer(
    storage: &Arc<dyn Storage + Send + Sync>,
    mempool: &Arc<Mempool>,
    block: &Block,
) -> Result<()> {
    storage.store_block(block.clone())?;

    for tx in &block.transactions {
        let hash_hex = hex_encode(tx.hash());
        if let Err(err) = mempool.remove_transaction(&hash_hex) {
            debug!(
                "Failed to prune transaction {} after importing block: {}",
                hash_hex, err
            );
        }
    }

    Ok(())
}

fn persist_transaction_from_peer(
    storage: &Arc<dyn Storage + Send + Sync>,
    mempool: &Arc<Mempool>,
    consensus: Option<ConsensusHandle>,
    tx: &Transaction,
) -> Result<()> {
    storage.store_transaction(tx.clone())?;

    match mempool.add_transaction(tx.clone()) {
        Ok(true) => {}
        Ok(false) => debug!(
            "Duplicate transaction from peer ignored: {}",
            hex_encode(tx.hash())
        ),
        Err(err) => return Err(err),
    }

    if let Some(handle) = consensus {
        if let Err(err) = handle.submit_transaction(tx.clone()) {
            warn!(
                "Consensus rejected inbound transaction {}: {}",
                hex_encode(tx.hash()),
                err
            );
        }
    }

    Ok(())
}

fn init_metrics(config: &AppConfig) -> Option<PrometheusHandle> {
    if !config.prometheus_enabled {
        info!("Prometheus metrics exporter disabled via configuration");
        return None;
    }

    match PrometheusBuilder::new().install_recorder() {
        Ok(handle) => {
            info!("Prometheus metrics exporter registered");
            describe_gauge!("node_health", "Overall node health indicator (1 = healthy)");
            describe_gauge!(
                "consensus_round",
                "Current consensus round number observed by the node"
            );
            describe_gauge!(
                "mempool_size",
                "Current number of transactions pending in the mempool"
            );
            Some(handle)
        }
        Err(err) => {
            warn!("Failed to install Prometheus metrics exporter: {}", err);
            None
        }
    }
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

fn build_dlc_ai_status_handle() -> Option<AiStatusHandle> {
    match catch_unwind(AssertUnwindSafe(|| {
        DlcConsensus::new(AiDlcConfig::default())
    })) {
        Ok(consensus) => {
            let consensus = Arc::new(Mutex::new(consensus));
            Some(AiStatusHandle::new({
                let consensus = consensus.clone();
                move || {
                    let consensus = consensus.clone();
                    async move {
                        let snapshot = {
                            let guard = consensus.lock().await;
                            guard.ai_status()
                        };
                        snapshot
                    }
                }
            }))
        }
        Err(_) => {
            warn!("Failed to initialize DLC AI status monitor; /ai/status will report disabled");
            None
        }
    }
}
