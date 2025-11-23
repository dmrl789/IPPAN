use anyhow::{anyhow, Result};
use clap::{value_parser, Arg, ArgAction, Command};
use config::{Config, File};
use hex::encode as hex_encode;
use ippan_consensus::{
    DLCConfig, DLCIntegratedConsensus, PoAConfig, PoAConsensus, Validator, VALIDATOR_BOND_AMOUNT,
};
use ippan_consensus_dlc::{DlcConfig as AiDlcConfig, DlcConsensus};
use ippan_files::{dht::StubFileDhtService, FileDhtService, FileStorage, MemoryFileStorage};
use ippan_l1_handle_anchors::L1HandleAnchorStorage;
use ippan_l2_handle_registry::{HandleDhtService, L2HandleRegistry, StubHandleDhtService};
use ippan_mempool::Mempool;
use ippan_p2p::{
    ChaosConfig, DhtConfig, HttpP2PNetwork, IpnDhtService, Libp2pConfig, Libp2pFileDhtService,
    Libp2pHandleDhtService, Libp2pNetwork, Multiaddr, NetworkEvent, P2PConfig, P2PLimits,
};
use ippan_rpc::server::ConsensusHandle;
use ippan_rpc::{start_server, AiStatusHandle, AppState, L2Config};
use ippan_security::{SecurityConfig as RpcSecurityConfig, SecurityManager as RpcSecurityManager};
use ippan_storage::{export_snapshot, import_snapshot, SledStorage, Storage};
use ippan_types::{
    ippan_time_init, ippan_time_now, Block, HashTimer, IppanTimeMicros, Transaction,
};
use metrics::{describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{IpAddr, TcpListener};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod version;

use version::{git_commit_hash, IPPAN_VERSION};

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

impl fmt::Display for FileDhtMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            FileDhtMode::Stub => "stub",
            FileDhtMode::Libp2p => "libp2p",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandleDhtMode {
    Stub,
    Libp2p,
}

impl HandleDhtMode {
    fn from_env(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "libp2p" => HandleDhtMode::Libp2p,
            _ => HandleDhtMode::Stub,
        }
    }
}

impl fmt::Display for HandleDhtMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            HandleDhtMode::Stub => "stub",
            HandleDhtMode::Libp2p => "libp2p",
        };
        f.write_str(value)
    }
}

/// Application configuration
#[derive(Debug, Clone)]
struct AppConfig {
    // Node identity
    node_id: String,
    validator_id: [u8; 32],
    network_id: String,

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
    chaos_drop_outbound_prob: u16,
    chaos_drop_inbound_prob: u16,
    chaos_extra_latency_ms_min: u64,
    chaos_extra_latency_ms_max: u64,

    // File DHT
    file_dht_mode: FileDhtMode,
    file_dht_listen_multiaddrs: Vec<String>,
    file_dht_bootstrap_multiaddrs: Vec<String>,
    handle_dht_mode: HandleDhtMode,

    // Unified UI
    unified_ui_dist_dir: Option<PathBuf>,

    // Security
    enable_security: bool,

    // Observability
    prometheus_enabled: bool,

    // Logging
    log_level: String,
    log_format: String,

    // Process management
    pid_file: Option<PathBuf>,

    // Development
    dev_mode: bool,
}

impl AppConfig {
    fn load(config_path: Option<&str>) -> Result<Self> {
        // Load from optional file first (so env vars can override)
        let mut builder = Config::builder();

        if let Some(path) = config_path {
            builder = builder.add_source(File::from(Path::new(path)));
        }

        builder = builder.add_source(config::Environment::with_prefix("IPPAN"));

        let config = builder.build()?;

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

        let file_dht_mode_value = get_string_value(&config, &["FILE_DHT_MODE", "dht.mode"])
            .unwrap_or_else(|| "stub".to_string());
        let file_dht_mode = FileDhtMode::from_env(&file_dht_mode_value);

        let handle_dht_mode_value =
            get_string_value(&config, &["HANDLE_DHT_MODE", "dht.handle_mode"])
                .unwrap_or_else(|| "stub".to_string());
        let handle_dht_mode = HandleDhtMode::from_env(&handle_dht_mode_value);

        let mut file_dht_listen_multiaddrs: Vec<String> = get_string_value(
            &config,
            &["FILE_DHT_LIBP2P_LISTEN", "dht.listen_multiaddrs"],
        )
        .unwrap_or_else(|| "/ip4/0.0.0.0/tcp/9100".to_string())
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
        if file_dht_listen_multiaddrs.is_empty() {
            file_dht_listen_multiaddrs.push("/ip4/0.0.0.0/tcp/9100".to_string());
        }

        let file_dht_bootstrap_multiaddrs: Vec<String> = get_string_value(
            &config,
            &["FILE_DHT_LIBP2P_BOOTSTRAP", "dht.bootstrap_multiaddrs"],
        )
        .unwrap_or_default()
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

        let dev_mode = get_bool_value(&config, &["DEV_MODE", "rpc.dev_mode"], false);

        let default_rpc_host = if dev_mode {
            "0.0.0.0".to_string()
        } else {
            "127.0.0.1".to_string()
        };

        Ok(Self {
            node_id: get_string_value(&config, &["NODE_ID", "node.id"])
                .unwrap_or_else(|| "ippan_node".to_string()),
            validator_id,
            network_id: get_string_value(&config, &["NETWORK_ID", "network.id", "network_id"])
                .unwrap_or_else(|| "ippan-devnet".to_string()),
            consensus_mode: get_string_value(&config, &["CONSENSUS_MODE", "consensus.mode"])
                .unwrap_or_else(|| "POA".to_string()),
            enable_dlc: get_bool_value(&config, &["ENABLE_DLC", "consensus.enable_dlc"], false),
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
            rpc_host: get_string_value(&config, &["RPC_HOST", "rpc.bind", "rpc.host"])
                .unwrap_or(default_rpc_host),
            rpc_port: get_string_value(&config, &["RPC_PORT", "rpc.port"])
                .unwrap_or_else(|| "8080".to_string())
                .parse()?,
            p2p_host: get_string_value(&config, &["P2P_HOST", "p2p.bind", "p2p.host"])
                .unwrap_or_else(|| "0.0.0.0".to_string()),
            p2p_port: get_string_value(&config, &["P2P_PORT", "p2p.port"])
                .unwrap_or_else(|| "9000".to_string())
                .parse()?,
            data_dir: get_string_value(&config, &["DATA_DIR", "storage.data_dir"])
                .unwrap_or_else(|| "./data".to_string()),
            db_path: get_string_value(&config, &["DB_PATH", "storage.db_path"])
                .unwrap_or_else(|| "./data/db".to_string()),
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
            bootstrap_nodes: get_string_value(&config, &["BOOTSTRAP_NODES", "p2p.bootstrap_nodes"])
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            max_peers: get_string_value(&config, &["MAX_PEERS", "p2p.max_peers"])
                .unwrap_or_else(|| "50".to_string())
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
            chaos_drop_outbound_prob: config
                .get_string("CHAOS_DROP_OUTBOUND_PROB")
                .unwrap_or_else(|_| "0".to_string())
                .parse()?,
            chaos_drop_inbound_prob: config
                .get_string("CHAOS_DROP_INBOUND_PROB")
                .unwrap_or_else(|_| "0".to_string())
                .parse()?,
            chaos_extra_latency_ms_min: config
                .get_string("CHAOS_EXTRA_LATENCY_MS_MIN")
                .unwrap_or_else(|_| "0".to_string())
                .parse()?,
            chaos_extra_latency_ms_max: config
                .get_string("CHAOS_EXTRA_LATENCY_MS_MAX")
                .unwrap_or_else(|_| "0".to_string())
                .parse()?,
            file_dht_mode,
            file_dht_listen_multiaddrs,
            file_dht_bootstrap_multiaddrs,
            handle_dht_mode,
            unified_ui_dist_dir,
            enable_security: get_bool_value(&config, &["ENABLE_SECURITY", "security.enable"], true),
            prometheus_enabled: config.get_bool("PROMETHEUS_ENABLED").unwrap_or(true),
            log_level: config
                .get_string("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            log_format: config
                .get_string("LOG_FORMAT")
                .unwrap_or_else(|_| "pretty".to_string()),
            dev_mode,
            pid_file: get_string_value(&config, &["PID_FILE", "node.pid_file"]).map(PathBuf::from),
        })
    }
}

fn get_string_value(config: &Config, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        config
            .get_string(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn get_bool_value(config: &Config, keys: &[&str], default: bool) -> bool {
    for key in keys {
        if let Ok(value) = config.get_bool(key) {
            return value;
        }
        if let Ok(raw) = config.get_string(key) {
            if let Ok(parsed) = raw.parse::<bool>() {
                return parsed;
            }
        }
    }
    default
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

fn load_config_with_overrides(matches: &clap::ArgMatches) -> Result<AppConfig> {
    let config_path = matches
        .get_one::<String>("config")
        .map(|value| value.as_str());
    let mut config = AppConfig::load(config_path)?;
    apply_overrides(matches, &mut config);

    if config.pid_file.is_none() {
        config.pid_file = Some(PathBuf::from(&config.data_dir).join("ippan-node.pid"));
    }

    Ok(config)
}

fn apply_overrides(matches: &clap::ArgMatches, config: &mut AppConfig) {
    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.data_dir = data_dir.clone();
        config.db_path = format!("{data_dir}/db");
    }

    if let Some(network) = matches.get_one::<String>("network") {
        config.network_id = network.clone();
    }

    if let Some(log_level) = matches.get_one::<String>("log-level") {
        config.log_level = log_level.clone();
    }

    if let Some(log_format) = matches.get_one::<String>("log-format") {
        config.log_format = log_format.clone();
    }

    if let Some(rpc_host) = matches.get_one::<String>("rpc-host") {
        config.rpc_host = rpc_host.clone();
    }

    if let Some(rpc_port) = matches.get_one::<u16>("rpc-port") {
        config.rpc_port = *rpc_port;
    }

    if let Some(p2p_port) = matches.get_one::<u16>("p2p-port") {
        config.p2p_port = *p2p_port;
    }

    if matches.get_flag("disable-metrics") {
        config.prometheus_enabled = false;
    }

    if let Some(pid_file) = matches.get_one::<String>("pid-file") {
        config.pid_file = Some(PathBuf::from(pid_file));
    }

    if matches.get_flag("dev") {
        config.dev_mode = true;
        config.log_level = "debug".to_string();
        config.log_format = "pretty".to_string();
        if config.rpc_host == "127.0.0.1" {
            config.rpc_host = "0.0.0.0".to_string();
        }
    }
}

struct PidFileGuard {
    path: PathBuf,
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn write_pid_file(path: &Path) -> Result<PidFileGuard> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let existing = fs::read_to_string(path).unwrap_or_default();
        anyhow::bail!(
            "PID file {} already exists (contents: {}); stop the running node or remove the file",
            path.display(),
            existing.trim()
        );
    }

    fs::write(path, std::process::id().to_string())?;
    Ok(PidFileGuard {
        path: path.to_path_buf(),
    })
}

fn stop_node(config: &AppConfig) -> Result<()> {
    let pid_path = config
        .pid_file
        .clone()
        .unwrap_or_else(|| PathBuf::from(&config.data_dir).join("ippan-node.pid"));

    if !pid_path.exists() {
        anyhow::bail!(
            "PID file {} not found; is the node running?",
            pid_path.display()
        );
    }

    let pid_raw = fs::read_to_string(&pid_path)?;
    let pid: i32 = pid_raw
        .trim()
        .parse()
        .map_err(|_| anyhow!("Invalid PID contents in {}", pid_path.display()))?;

    let status = StdCommand::new("kill")
        .arg("-INT")
        .arg(pid.to_string())
        .status()?;

    if status.success() {
        println!("Sent SIGINT to IPPAN node process {pid}");
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to signal PID {} (exit status: {:?})",
            pid,
            status.code()
        )
    }
}

async fn check_status(config: &AppConfig, health_path: &str) -> Result<()> {
    let mut path = health_path.to_string();
    if !path.starts_with('/') {
        path = format!("/{path}");
    }
    let url = format!("http://{}:{}{}", config.rpc_host, config.rpc_port, path);
    let response = reqwest::Client::new().get(&url).send().await?;
    let status = response.status();
    let body = response.text().await?;
    println!("GET {url} -> {status}");
    println!("{body}");
    if status.is_success() {
        Ok(())
    } else {
        anyhow::bail!("Health check failed with status {status}")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("ippan-node")
        .version(IPPAN_VERSION)
        .about("IPPAN Blockchain Node")
        .disable_version_flag(true)
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .global(true),
        )
        .arg(
            Arg::new("data-dir")
                .short('d')
                .long("data-dir")
                .value_name("DIR")
                .help("Data directory")
                .global(true),
        )
        .arg(
            Arg::new("dev")
                .long("dev")
                .action(ArgAction::SetTrue)
                .help("Run in development mode")
                .global(true),
        )
        .arg(
            Arg::new("version_flag")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue)
                .help("Print detailed version information and exit")
                .global(true),
        )
        .arg(
            Arg::new("check")
                .long("check")
                .action(ArgAction::SetTrue)
                .help("Run configuration and environment self-checks, then exit")
                .global(true),
        )
        .arg(
            Arg::new("network")
                .long("network")
                .value_name("NETWORK")
                .help("Network identifier (e.g. localnet or testnet)")
                .global(true),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .value_parser(["trace", "debug", "info", "warn", "error"])
                .help("Override the log level")
                .global(true),
        )
        .arg(
            Arg::new("log-format")
                .long("log-format")
                .value_name("FORMAT")
                .value_parser(["pretty", "json"])
                .help("Select log output format")
                .global(true),
        )
        .arg(
            Arg::new("rpc-host")
                .long("rpc-host")
                .value_name("HOST")
                .help("Override RPC bind host (defaults to config value)")
                .global(true),
        )
        .arg(
            Arg::new("rpc-port")
                .long("rpc-port")
                .value_name("PORT")
                .value_parser(value_parser!(u16))
                .help("Override RPC port")
                .global(true),
        )
        .arg(
            Arg::new("p2p-port")
                .long("p2p-port")
                .value_name("PORT")
                .value_parser(value_parser!(u16))
                .help("Override P2P port")
                .global(true),
        )
        .arg(
            Arg::new("disable-metrics")
                .long("disable-metrics")
                .action(ArgAction::SetTrue)
                .help("Disable the Prometheus metrics endpoint")
                .global(true),
        )
        .arg(
            Arg::new("pid-file")
                .long("pid-file")
                .value_name("FILE")
                .help("PID file to use for start/stop coordination")
                .global(true),
        )
        .subcommand(
            Command::new("start").about("Start the IPPAN node using the provided configuration"),
        )
        .subcommand(
            Command::new("status")
                .about("Check the /health endpoint for a running node")
                .arg(
                    Arg::new("health-path")
                        .long("health-path")
                        .value_name("PATH")
                        .default_value("/health")
                        .help("Health endpoint path to query"),
                ),
        )
        .subcommand(
            Command::new("stop")
                .about("Send a SIGINT to a running node based on its PID file")
                .arg(
                    Arg::new("pid-file")
                        .long("pid-file")
                        .value_name("FILE")
                        .help("PID file to read when stopping the node"),
                ),
        )
        .subcommand(
            Command::new("snapshot")
                .about("Export or import on-disk snapshots")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("export")
                        .about("Export snapshot data to a directory")
                        .arg(
                            Arg::new("dir")
                                .long("dir")
                                .value_name("PATH")
                                .required(true)
                                .help("Directory to write the snapshot into"),
                        ),
                )
                .subcommand(
                    Command::new("import")
                        .about("Import snapshot data from a directory")
                        .arg(
                            Arg::new("dir")
                                .long("dir")
                                .value_name("PATH")
                                .required(true)
                                .help("Directory previously created via `snapshot export`"),
                        ),
                ),
        )
        .get_matches();

    if let Some(status_matches) = matches.subcommand_matches("status") {
        let config = load_config_with_overrides(status_matches)?;
        let health_path = status_matches
            .get_one::<String>("health-path")
            .map(|value| value.as_str())
            .unwrap_or("/health");
        check_status(&config, health_path).await?;
        return Ok(());
    }

    if let Some(stop_matches) = matches.subcommand_matches("stop") {
        let config = load_config_with_overrides(stop_matches)?;
        stop_node(&config)?;
        return Ok(());
    }

    if let Some(snapshot_matches) = matches.subcommand_matches("snapshot") {
        let config = load_config_with_overrides(snapshot_matches)?;
        handle_snapshot_subcommand(snapshot_matches, &config)?;
        return Ok(());
    }

    let start_matches = matches.subcommand_matches("start").unwrap_or(&matches);

    let mut config = load_config_with_overrides(start_matches)?;
    let mut dev_mode_forced_off = false;

    if cfg!(feature = "production") && config.dev_mode {
        dev_mode_forced_off = true;
        config.dev_mode = false;
    }

    if start_matches.get_flag("version_flag") {
        print_version_info(&config);
        return Ok(());
    }

    fs::create_dir_all(&config.data_dir)?;
    if let Some(parent) = Path::new(&config.db_path).parent() {
        fs::create_dir_all(parent)?;
    }

    if start_matches.get_flag("check") {
        run_self_check(&config)?;
        return Ok(());
    }

    // Initialize logging
    init_logging(&config)?;

    if dev_mode_forced_off {
        warn!(
            "Production feature enabled: development-only endpoints are disabled regardless of flags"
        );
    }

    let _pid_guard = if let Some(pid_file) = &config.pid_file {
        Some(write_pid_file(pid_file)?)
    } else {
        None
    };

    // Initialize metrics exporter
    let prometheus_handle = init_metrics(&config);

    // Initialize IPPAN Time service
    ippan_time_init();
    info!("IPPAN Time service initialized");

    info!("Starting IPPAN node: {}", config.node_id);
    info!("Validator ID: {}", hex::encode(config.validator_id));
    info!("Network ID: {}", config.network_id);
    info!("Data directory: {}", config.data_dir);
    info!("Development mode: {}", config.dev_mode);

    if !config.dev_mode {
        if let Ok(ip) = config.rpc_host.parse::<IpAddr>() {
            if ip.is_unspecified() {
                warn!(
                    "RPC host {} binds to all interfaces outside dev mode; consider setting IPPAN_RPC_HOST=127.0.0.1 and fronting the API with a reverse proxy or firewall",
                    config.rpc_host
                );
            }
        }
    }

    // Create data directory
    fs::create_dir_all(&config.data_dir)?;

    // Initialize storage
    let storage = Arc::new(SledStorage::new(&config.db_path)?);
    storage.set_network_id(&config.network_id)?;
    storage.initialize()?;
    info!("Storage initialized at {}", config.db_path);

    let handle_registry = Arc::new(L2HandleRegistry::new());
    let handle_anchors = Arc::new(L1HandleAnchorStorage::new());

    let need_ipn_dht_network = matches!(config.file_dht_mode, FileDhtMode::Libp2p)
        || matches!(config.handle_dht_mode, HandleDhtMode::Libp2p);
    let ipn_dht_backend: Option<Arc<IpnDhtService>> = if need_ipn_dht_network {
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
        for topic in ["ippan/files", "ippan/handles"] {
            if !libp2p_config
                .gossip_topics
                .iter()
                .any(|existing| existing == topic)
            {
                libp2p_config.gossip_topics.push(topic.to_string());
            }
        }
        match Libp2pNetwork::new(libp2p_config) {
            Ok(network) => {
                let network = Arc::new(network);
                let addresses = network.listen_addresses();
                info!("IPNDHT libp2p listening on {:?}", addresses);
                Some(Arc::new(IpnDhtService::new(Some(network))))
            }
            Err(err) => {
                warn!(
                    "Failed to initialise libp2p IPNDHT (fallback to stub services): {}",
                    err
                );
                None
            }
        }
    } else {
        None
    };

    let file_dht: Arc<dyn FileDhtService> = match config.file_dht_mode {
        FileDhtMode::Stub => {
            info!("File DHT mode set to stub");
            Arc::new(StubFileDhtService::new())
        }
        FileDhtMode::Libp2p => {
            info!("File DHT mode set to libp2p");
            if let Some(ipn) = ipn_dht_backend.clone() {
                Arc::new(Libp2pFileDhtService::new(ipn))
            } else {
                warn!("IPNDHT backend unavailable; falling back to stub File DHT");
                Arc::new(StubFileDhtService::new())
            }
        }
    };

    let handle_dht: Arc<dyn HandleDhtService> = match config.handle_dht_mode {
        HandleDhtMode::Stub => {
            info!("Handle DHT mode set to stub");
            Arc::new(StubHandleDhtService::new())
        }
        HandleDhtMode::Libp2p => {
            info!("Handle DHT mode set to libp2p");
            if let Some(ipn) = ipn_dht_backend.clone() {
                Arc::new(Libp2pHandleDhtService::new(ipn))
            } else {
                warn!("IPNDHT backend unavailable; falling back to stub Handle DHT");
                Arc::new(StubHandleDhtService::new())
            }
        }
    };

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
            Some(handle_dht.clone()),
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
            Some(handle_dht.clone()),
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
    let dht_config = DhtConfig {
        bootstrap_peers: config.bootstrap_nodes.clone(),
        public_host: config.p2p_public_host.clone(),
        enable_upnp: config.p2p_enable_upnp,
        external_ip_services: config.p2p_external_ip_services.clone(),
        announce_interval: Duration::from_secs(config.peer_announce_interval_secs),
    };

    let p2p_config = P2PConfig {
        listen_address: listen_address.clone(),
        max_peers: config.max_peers,
        peer_discovery_interval: Duration::from_secs(config.peer_discovery_interval_secs),
        message_timeout: Duration::from_secs(10),
        retry_attempts: 3,
        dht: dht_config,
        chaos: ChaosConfig {
            drop_outbound_prob: config.chaos_drop_outbound_prob,
            drop_inbound_prob: config.chaos_drop_inbound_prob,
            extra_latency_ms_min: config.chaos_extra_latency_ms_min,
            extra_latency_ms_max: config.chaos_extra_latency_ms_max,
        },
        limits: P2PLimits::default(),
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

    let app_state = AppState {
        storage: storage.clone(),
        start_time,
        peer_count: peer_count.clone(),
        p2p_network: Some(p2p_network_arc.clone()),
        tx_sender: Some(tx_sender.clone()),
        node_id: config.node_id.clone(),
        consensus_mode: config.consensus_mode.clone(),
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
        dht_file_mode: config.file_dht_mode.to_string(),
        dev_mode: config.dev_mode,
        handle_registry: handle_registry.clone(),
        handle_anchors: handle_anchors.clone(),
        handle_dht: Some(handle_dht.clone()),
        dht_handle_mode: config.handle_dht_mode.to_string(),
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
                gauge!("p2p_connected_peers").set(count as f64);
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
            describe_counter!(
                "rpc_requests_total",
                "Total RPC requests processed, labeled by method and path"
            );
            describe_counter!(
                "rpc_requests_failed_total",
                "Total RPC requests that returned client/server errors"
            );
            describe_counter!(
                "p2p_peers_connected_total",
                "Total peers observed joining the node"
            );
            describe_counter!("p2p_peers_dropped_total", "Total peers removed or dropped");
            describe_gauge!("node_health", "Overall node health indicator (1 = healthy)");
            describe_gauge!(
                "consensus_round",
                "Current consensus round number observed by the node"
            );
            describe_gauge!(
                "mempool_size",
                "Current number of transactions pending in the mempool"
            );
            describe_gauge!(
                "p2p_connected_peers",
                "Number of peers currently connected via HTTP/libp2p"
            );
            describe_gauge!(
                "node_uptime_seconds",
                "Node uptime in seconds since process start"
            );
            describe_gauge!(
                "consensus_finalized_round",
                "Last finalized consensus round id"
            );
            describe_gauge!(
                "node_build_info",
                "Build metadata gauge (labels: version, commit)"
            );
            gauge!("node_build_info", "version" => IPPAN_VERSION, "commit" => git_commit_hash())
                .set(1.0);
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

fn print_version_info(config: &AppConfig) {
    let mode = consensus_mode_label(config);
    println!(
        "IPPAN {} (commit {}) [{}]",
        IPPAN_VERSION,
        git_commit_hash(),
        mode
    );
}

fn consensus_mode_label(config: &AppConfig) -> &'static str {
    if config.consensus_mode.eq_ignore_ascii_case("DLC") || config.enable_dlc {
        "DLC"
    } else {
        "PoA"
    }
}

fn run_self_check(config: &AppConfig) -> Result<()> {
    println!("Running IPPAN node self-check...");
    let mut issues = Vec::new();

    if config.node_id.trim().is_empty() {
        issues.push("NODE_ID must not be empty".to_string());
    }

    let consensus = config.consensus_mode.to_uppercase();
    if consensus != "POA" && consensus != "DLC" {
        issues.push(format!(
            "Invalid CONSENSUS_MODE '{}'; expected 'PoA' or 'DLC'",
            config.consensus_mode
        ));
    }

    if config.rpc_port == 0 {
        issues.push("RPC_PORT must be greater than zero".to_string());
    }

    if config.p2p_port == 0 {
        issues.push("P2P_PORT must be greater than zero".to_string());
    }

    if config.rpc_port == config.p2p_port {
        issues.push("RPC_PORT and P2P_PORT must be different".to_string());
    }

    if let Err(err) = ensure_port_available(&config.rpc_host, config.rpc_port, "RPC") {
        issues.push(err);
    }

    if let Err(err) = ensure_port_available(&config.p2p_host, config.p2p_port, "P2P") {
        issues.push(err);
    }

    if let Err(err) = ensure_storage_directory(&config.data_dir) {
        issues.push(err);
    }

    if let Some(parent) = Path::new(&config.db_path).parent() {
        if !parent.exists() {
            issues.push(format!(
                "Database directory {} does not exist",
                parent.display()
            ));
        }
    } else {
        issues.push(format!("DB_PATH '{}' is invalid", config.db_path));
    }

    if issues.is_empty() {
        println!("OK");
        Ok(())
    } else {
        for issue in &issues {
            eprintln!("- {issue}");
        }
        anyhow::bail!("self-check failed")
    }
}

fn handle_snapshot_subcommand(matches: &clap::ArgMatches, config: &AppConfig) -> Result<()> {
    match matches.subcommand() {
        Some(("export", export_matches)) => {
            let dir = export_matches
                .get_one::<String>("dir")
                .ok_or_else(|| anyhow!("--dir is required"))?;
            let dir_path = PathBuf::from(dir);
            let storage = SledStorage::new(&config.db_path)?;
            storage.set_network_id(&config.network_id)?;
            let manifest = export_snapshot(&storage, &dir_path)?;
            println!(
                "Snapshot exported to {} (height {}, accounts {}, files {})",
                dir_path.display(),
                manifest.height,
                manifest.accounts_count,
                manifest.files_count
            );
            println!(
                "Network: {} • Payments: {} • Blocks: {}",
                manifest.network_id, manifest.payments_count, manifest.blocks_count
            );
            Ok(())
        }
        Some(("import", import_matches)) => {
            let dir = import_matches
                .get_one::<String>("dir")
                .ok_or_else(|| anyhow!("--dir is required"))?;
            let dir_path = PathBuf::from(dir);
            let mut storage = SledStorage::new(&config.db_path)?;
            storage.set_network_id(&config.network_id)?;
            let manifest = import_snapshot(&mut storage, &dir_path)?;
            println!(
                "Snapshot imported from {} (height {}, accounts {}, files {})",
                dir_path.display(),
                manifest.height,
                manifest.accounts_count,
                manifest.files_count
            );
            println!(
                "Network: {} • Payments: {} • Blocks: {}",
                manifest.network_id, manifest.payments_count, manifest.blocks_count
            );
            Ok(())
        }
        _ => Err(anyhow!("Unsupported snapshot command")),
    }
}

fn ensure_port_available(host: &str, port: u16, label: &str) -> Result<(), String> {
    let addr = format!("{host}:{port}");
    match TcpListener::bind(&addr) {
        Ok(listener) => drop(listener),
        Err(err) => {
            return Err(format!(
                "{label} port {addr} is not available for binding: {err}"
            ))
        }
    }
    Ok(())
}

fn ensure_storage_directory(path: &str) -> Result<(), String> {
    let dir = Path::new(path);
    if !dir.exists() {
        return Err(format!(
            "Storage directory {} does not exist; create it before starting the node",
            dir.display()
        ));
    }
    if !dir.is_dir() {
        return Err(format!("Storage path {} is not a directory", dir.display()));
    }

    let probe = dir.join(".ippan_write_test");
    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(mut file) => {
            if let Err(err) = file.write_all(b"ok") {
                return Err(format!("Unable to write into {}: {}", dir.display(), err));
            }
        }
        Err(err) => {
            return Err(format!(
                "Unable to open {} for writing: {}",
                dir.display(),
                err
            ));
        }
    }
    let _ = fs::remove_file(&probe);
    Ok(())
}
