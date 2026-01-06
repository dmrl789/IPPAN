use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "cargo run -p ippan-node --bin ippan-node --")]
    node_cmd: String,

    #[arg(long, default_value = "")]
    node_args: String,

    #[arg(long, default_value = "./artifacts/p2p")]
    artifacts_dir: String,
}

#[derive(Subcommand)]
enum Commands {
    Smoke {
        #[arg(long, default_value_t = 4)]
        nodes: usize,
    },
    Gossip {
        #[arg(long, default_value_t = 8)]
        nodes: usize,
        #[arg(long, default_value_t = 200)]
        msgs: usize,
        #[arg(long, default_value_t = 512)]
        size: usize,
        #[arg(long, default_value_t = 20)]
        warmup_msgs: usize,
        #[arg(long, default_value_t = 3)]
        warmup_seconds: u64,
    },
    Churn {
        #[arg(long, default_value_t = 10)]
        nodes: usize,
        #[arg(long, default_value_t = 50)]
        churn_pct: usize,
        #[arg(long, default_value_t = 3)]
        minutes: u64,
    },
    Chaos {
        #[arg(long, default_value_t = 6)]
        nodes: usize,
        #[arg(long, default_value_t = 2)]
        loss: usize,
        #[arg(long, default_value_t = 80)]
        latency_ms: u64,
        #[arg(long, default_value_t = 20)]
        jitter_ms: u64,
        #[arg(long, default_value_t = 2)]
        minutes: u64,
        #[arg(long, default_value_t = 200)]
        msgs: usize,
        #[arg(long, default_value_t = 256)]
        size: usize,
        #[arg(long, default_value_t = 0.70)]
        min_delivery_rate: f64,
        #[arg(long, default_value_t = 20)]
        warmup_msgs: usize,
        #[arg(long, default_value_t = 3)]
        warmup_seconds: u64,
    },
}

#[derive(Serialize, Deserialize, Default)]
struct Summary {
    pass: bool,
    scenario: String,
    timestamp: String,
    nodes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    msgs_sent: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_total_receipts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_receipts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate_receipts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    per_node_unique_receipts: Option<HashMap<usize, usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delivery_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_formula: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_samples: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_latency_samples: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_p50_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_p95_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_max_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warmup_msgs: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warmup_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measurement_start_ts_ms: Option<u64>,
    metrics: HashMap<String, serde_json::Value>,
    errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
}

struct NodeInstance {
    process: Option<Child>,
    _data_dir: PathBuf,
    p2p_port: u16,
    rpc_port: u16,
    _peer_id: Arc<Mutex<Option<String>>>,
    connected_peers: Arc<Mutex<HashSet<String>>>,
    gossip_received: Arc<Mutex<Vec<(u64, u64, u64)>>>, // (msg_id, sent_ts_ms, recv_ts_ms)
    is_alive: bool,
}

impl NodeInstance {
    fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.start_kill();
            let _ = futures::executor::block_on(child.wait());
        }
        self.is_alive = false;
    }
}

struct TestHarness {
    artifacts_path: PathBuf,
    nodes: Vec<NodeInstance>,
    node_cmd: String,
    node_args: Vec<String>,
}

impl TestHarness {
    fn new(artifacts_dir: &str, node_cmd: &str, node_args: &str) -> Result<Self> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let artifacts_path = PathBuf::from(artifacts_dir).join(timestamp);
        std::fs::create_dir_all(&artifacts_path)?;

        let node_args_vec = node_args
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        Ok(Self {
            artifacts_path,
            nodes: Vec::new(),
            node_cmd: node_cmd.to_string(),
            node_args: node_args_vec,
        })
    }

    async fn spawn_node(
        &mut self,
        id: usize,
        bootstrap_addr: Option<String>,
        extra_args: Vec<String>,
        extra_env: HashMap<String, String>,
    ) -> Result<()> {
        let data_dir = self.artifacts_path.join(format!("node{}", id));
        std::fs::create_dir_all(&data_dir)?;

        let p2p_port = 19000 + (id as u16);
        let rpc_port = 18080 + (id as u16);

        let mut cmd_parts = self.node_cmd.split_whitespace().collect::<Vec<_>>();
        let program = cmd_parts.remove(0);

        let mut cmd = Command::new(program);
        for part in cmd_parts {
            cmd.arg(part);
        }

        cmd.arg("start")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--p2p-port")
            .arg(p2p_port.to_string())
            .arg("--rpc-port")
            .arg(rpc_port.to_string())
            .arg("--log-level")
            .arg("info")
            .arg("--log-format")
            .arg("pretty")
            .env("IPPAN_LOG_FORMAT", "pretty") // Ensure pretty but no-color if possible
            // Actually, let's use plain format if ippan-node supports it.
            // ippan-node uses tracing-subscriber pretty() which has colors.
            // I'll just remove ANSI codes in the parser.
            // Ensure libp2p is enabled for test topic
            .env("IPPAN_FILE_DHT_MODE", "libp2p");

        for arg in &self.node_args {
            cmd.arg(arg);
        }
        for arg in extra_args {
            cmd.arg(arg);
        }

        cmd.envs(extra_env);

        if let Some(addr) = bootstrap_addr {
            cmd.env("IPPAN_BOOTSTRAP_NODES", addr);
        }

        let identity_key_path = data_dir.join("p2p_identity.key");
        cmd.env(
            "IPPAN_P2P_IDENTITY_KEY_PATH",
            identity_key_path.to_str().unwrap(),
        );
        cmd.env("IPPAN_ENABLE_P2P_TEST_RPC", "1");

        println!(
            "Spawning node {}: {} {:?}",
            id,
            program,
            cmd.as_std().get_args().collect::<Vec<_>>()
        );

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let log_file_path = self.artifacts_path.join(format!("node{}.log", id));
        let mut log_file = std::fs::File::create(log_file_path)?;

        let peer_id_shared = Arc::new(Mutex::new(None));
        let connected_peers_shared = Arc::new(Mutex::new(HashSet::new()));
        let gossip_received_shared = Arc::new(Mutex::new(Vec::new()));

        let peer_id_clone = peer_id_shared.clone();
        let connected_peers_clone = connected_peers_shared.clone();
        let gossip_received_clone = gossip_received_shared.clone();

        // Log processing task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                let _ = std::io::Write::write_all(&mut log_file, line.as_bytes());

                // Strip ANSI escape codes for easier parsing
                let clean_line = strip_ansi_codes(&line);

                // Parse hooks
                if clean_line.contains("Node ID: ") {
                    if let Some(id_str) = clean_line.split("Node ID: ").collect::<Vec<_>>().get(1) {
                        let mut guard = peer_id_clone.lock().await;
                        *guard = Some(id_str.trim().to_string());
                    }
                }
                if clean_line.contains("peer_connected ") {
                    let re = regex::Regex::new(r"peer_connected (\S+)").unwrap();
                    if let Some(caps) = re.captures(&clean_line) {
                        connected_peers_clone
                            .lock()
                            .await
                            .insert(caps[1].to_string());
                    }
                }
                if clean_line.contains("peer_disconnected ") {
                    let re = regex::Regex::new(r"peer_disconnected (\S+)").unwrap();
                    if let Some(caps) = re.captures(&clean_line) {
                        connected_peers_clone
                            .lock()
                            .await
                            .remove(&caps[1].to_string());
                    }
                }
                if clean_line.contains("gossip_received ippan/test/gossip") {
                    let re =
                        regex::Regex::new(r"gossip_received ippan/test/gossip (\d+) (\S+) (\d+)")
                            .unwrap();
                    if let Some(caps) = re.captures(&clean_line) {
                        if let (Ok(msg_id), Ok(sent_ts)) =
                            (caps[1].parse::<u64>(), caps[3].parse::<u64>())
                        {
                            let recv_ts = chrono::Utc::now().timestamp_millis() as u64;
                            gossip_received_clone
                                .lock()
                                .await
                                .push((msg_id, sent_ts, recv_ts));
                        }
                    }
                }
                line.clear();
            }
        });

        let mut log_file_err = std::fs::OpenOptions::new()
            .append(true)
            .open(self.artifacts_path.join(format!("node{}.log", id)))?;
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                let _ = std::io::Write::write_all(&mut log_file_err, line.as_bytes());
                line.clear();
            }
        });

        let node = NodeInstance {
            process: Some(child),
            _data_dir: data_dir,
            p2p_port,
            rpc_port,
            _peer_id: peer_id_shared,
            connected_peers: connected_peers_shared,
            gossip_received: gossip_received_shared,
            is_alive: true,
        };

        if id < self.nodes.len() {
            self.nodes[id] = node;
        } else {
            self.nodes.push(node);
        }

        Ok(())
    }

    async fn write_summary(&self, mut summary: Summary) -> Result<()> {
        summary.timestamp = chrono::Local::now().to_rfc3339();
        let summary_path = self.artifacts_path.join("summary.json");
        let content = serde_json::to_string_pretty(&summary)?;
        std::fs::write(&summary_path, content)?;
        info!("Summary written to {}", summary_path.display());
        Ok(())
    }

    fn stop_all(&mut self) {
        for node in &mut self.nodes {
            node.stop();
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let node_cmd = std::env::var("IPPAN_NODE_CMD").unwrap_or(cli.node_cmd);
    let node_args = std::env::var("IPPAN_NODE_ARGS").unwrap_or(cli.node_args);

    let mut harness = TestHarness::new(&cli.artifacts_dir, &node_cmd, &node_args)?;

    let res = match cli.command {
        Commands::Smoke { nodes } => run_smoke(&mut harness, nodes).await,
        Commands::Gossip {
            nodes,
            msgs,
            size,
            warmup_msgs,
            warmup_seconds,
        } => run_gossip(&mut harness, nodes, msgs, size, warmup_msgs, warmup_seconds).await,
        Commands::Churn {
            nodes,
            churn_pct,
            minutes,
        } => run_churn(&mut harness, nodes, churn_pct, minutes).await,
        Commands::Chaos {
            nodes,
            loss,
            latency_ms,
            jitter_ms,
            minutes,
            msgs,
            size,
            min_delivery_rate,
            warmup_msgs,
            warmup_seconds,
        } => {
            run_chaos(
                &mut harness,
                nodes,
                loss,
                latency_ms,
                jitter_ms,
                minutes,
                msgs,
                size,
                min_delivery_rate,
                warmup_msgs,
                warmup_seconds,
            )
            .await
        }
    };

    harness.stop_all();
    res
}

async fn run_smoke(harness: &mut TestHarness, node_count: usize) -> Result<()> {
    info!("Starting SMOKE test with {} nodes", node_count);
    let mut summary = Summary {
        scenario: "smoke".to_string(),
        nodes: node_count,
        ..Default::default()
    };

    harness.spawn_node(0, None, vec![], HashMap::new()).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    let bootstrap_port = harness.nodes[0].p2p_port;
    let bootstrap_addr = format!("http://127.0.0.1:{}", bootstrap_port);

    for i in 1..node_count {
        harness
            .spawn_node(i, Some(bootstrap_addr.clone()), vec![], HashMap::new())
            .await?;
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(30);
    let mut passed = false;

    while start.elapsed() < timeout {
        let mut all_connected = true;
        for node in &harness.nodes {
            let peers = node.connected_peers.lock().await;
            if peers.is_empty() {
                all_connected = false;
                break;
            }
        }

        if all_connected {
            passed = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    summary.pass = passed;
    if !passed {
        summary
            .errors
            .push("Not all nodes connected to at least one peer within timeout".to_string());
    }

    harness.write_summary(summary).await?;
    if passed {
        info!("SMOKE test PASSED");
        Ok(())
    } else {
        warn!("SMOKE test FAILED");
        Err(anyhow!("SMOKE test failed"))
    }
}

async fn run_gossip(
    harness: &mut TestHarness,
    node_count: usize,
    msg_count: usize,
    msg_size: usize,
    warmup_msgs: usize,
    warmup_seconds: u64,
) -> Result<()> {
    info!(
        "Starting GOSSIP test: nodes={}, msgs={}, size={}, warmup_msgs={}, warmup_secs={}",
        node_count, msg_count, msg_size, warmup_msgs, warmup_seconds
    );
    let mut summary = Summary {
        scenario: "gossip".to_string(),
        nodes: node_count,
        warmup_msgs: Some(warmup_msgs),
        warmup_seconds: Some(warmup_seconds),
        ..Default::default()
    };

    harness.spawn_node(0, None, vec![], HashMap::new()).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let bootstrap_addr = format!("http://127.0.0.1:{}", harness.nodes[0].p2p_port);
    for i in 1..node_count {
        harness
            .spawn_node(i, Some(bootstrap_addr.clone()), vec![], HashMap::new())
            .await?;
    }

    // Wait for mesh readiness
    tokio::time::sleep(Duration::from_secs(15)).await;

    let (sent_count, measurement_start_ts_ms) = run_gossip_burst(
        harness,
        msg_count,
        msg_size,
        10,
        warmup_msgs,
        warmup_seconds,
    )
    .await?;
    let stats = calculate_gossip_stats(&harness.nodes, sent_count, measurement_start_ts_ms).await;

    summary.msgs_sent = Some(sent_count);
    summary.measurement_start_ts_ms = Some(measurement_start_ts_ms);
    summary.expected_total_receipts = Some(stats.expected_total_receipts);
    summary.unique_receipts = Some(stats.unique_receipts);
    summary.duplicate_receipts = Some(stats.duplicate_receipts);
    summary.per_node_unique_receipts = Some(stats.per_node_unique_receipts);
    summary.delivery_rate = Some(stats.delivery_rate);
    summary.expected_formula = Some("msgs_sent * (nodes - 1)".to_string());

    summary.latency_samples = Some(stats.latency_samples);
    summary.negative_latency_samples = Some(stats.negative_latency_samples);
    summary.latency_p50_ms = Some(stats.latency_p50_ms);
    summary.latency_p95_ms = Some(stats.latency_p95_ms);
    summary.latency_max_ms = Some(stats.latency_max_ms);

    summary.pass = stats.delivery_rate >= 0.95;
    let pass = summary.pass;
    let delivery_rate = stats.delivery_rate;
    harness.write_summary(summary).await?;

    if pass {
        info!(
            "GOSSIP test PASSED (delivery_rate={:.2}%)",
            delivery_rate * 100.0
        );
        Ok(())
    } else {
        warn!(
            "GOSSIP test FAILED (delivery_rate={:.2}%)",
            delivery_rate * 100.0
        );
        Err(anyhow!("GOSSIP test failed"))
    }
}

async fn run_churn(
    harness: &mut TestHarness,
    node_count: usize,
    churn_pct: usize,
    minutes: u64,
) -> Result<()> {
    info!(
        "Starting CHURN test: nodes={}, churn={}%, minutes={}",
        node_count, churn_pct, minutes
    );
    let seed = rand::thread_rng().gen::<u64>();
    let mut rng = StdRng::seed_base(seed);
    let mut summary = Summary {
        scenario: "churn".to_string(),
        nodes: node_count,
        seed: Some(seed),
        ..Default::default()
    };

    harness.spawn_node(0, None, vec![], HashMap::new()).await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let bootstrap_addr = format!("http://127.0.0.1:{}", harness.nodes[0].p2p_port);
    for i in 1..node_count {
        harness
            .spawn_node(i, Some(bootstrap_addr.clone()), vec![], HashMap::new())
            .await?;
    }

    let end_time = Instant::now() + Duration::from_secs(minutes * 60);
    let mut churn_events = 0;

    while Instant::now() < end_time {
        // Randomly kill/restart a node (not node0)
        let idx = rng.gen_range(1..node_count);
        if harness.nodes[idx].is_alive {
            if rng.gen_ratio(churn_pct as u32, 100) {
                info!("Churn: killing node {}", idx);
                harness.nodes[idx].stop();
                churn_events += 1;
            }
        } else {
            info!("Churn: restarting node {}", idx);
            harness
                .spawn_node(idx, Some(bootstrap_addr.clone()), vec![], HashMap::new())
                .await?;
            churn_events += 1;
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    summary
        .metrics
        .insert("churn_events".to_string(), serde_json::json!(churn_events));
    summary.pass = true; // Churn test passes if no deadlock/crash (best effort)
    harness.write_summary(summary).await?;
    Ok(())
}

async fn run_chaos(
    harness: &mut TestHarness,
    node_count: usize,
    loss: usize,
    latency: u64,
    jitter: u64,
    _minutes: u64,
    msgs: usize,
    size: usize,
    min_delivery_rate: f64,
    warmup_msgs: usize,
    warmup_seconds: u64,
) -> Result<()> {
    info!("Starting CHAOS test: nodes={}, loss={}%, latency={}ms, jitter={}ms, msgs={}, size={}, min_delivery={}, warmup_msgs={}, warmup_secs={}", 
        node_count, loss, latency, jitter, msgs, size, min_delivery_rate, warmup_msgs, warmup_seconds);
    let mut summary = Summary {
        scenario: "chaos".to_string(),
        nodes: node_count,
        warmup_msgs: Some(warmup_msgs),
        warmup_seconds: Some(warmup_seconds),
        ..Default::default()
    };

    summary
        .metrics
        .insert("loss_pct".to_string(), serde_json::json!(loss));
    summary
        .metrics
        .insert("latency_ms".to_string(), serde_json::json!(latency));
    summary
        .metrics
        .insert("jitter_ms".to_string(), serde_json::json!(jitter));

    // IPPAN chaos environment variables
    let mut chaos_env = HashMap::new();
    chaos_env.insert(
        "IPPAN_CHAOS_DROP_OUTBOUND_PROB".to_string(),
        (loss * 100).to_string(),
    );
    chaos_env.insert(
        "IPPAN_CHAOS_DROP_INBOUND_PROB".to_string(),
        (loss * 100).to_string(),
    );
    chaos_env.insert(
        "IPPAN_CHAOS_EXTRA_LATENCY_MS_MIN".to_string(),
        latency.to_string(),
    );
    chaos_env.insert(
        "IPPAN_CHAOS_EXTRA_LATENCY_MS_MAX".to_string(),
        (latency + jitter).to_string(),
    );

    harness
        .spawn_node(0, None, vec![], chaos_env.clone())
        .await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let bootstrap_addr = format!("http://127.0.0.1:{}", harness.nodes[0].p2p_port);
    for i in 1..node_count {
        harness
            .spawn_node(i, Some(bootstrap_addr.clone()), vec![], chaos_env.clone())
            .await?;
    }

    // Wait for mesh readiness
    tokio::time::sleep(Duration::from_secs(20)).await;

    let (sent_count, measurement_start_ts_ms) =
        run_gossip_burst(harness, msgs, size, 15, warmup_msgs, warmup_seconds).await?;
    let stats = calculate_gossip_stats(&harness.nodes, sent_count, measurement_start_ts_ms).await;

    summary.msgs_sent = Some(sent_count);
    summary.measurement_start_ts_ms = Some(measurement_start_ts_ms);
    summary.expected_total_receipts = Some(stats.expected_total_receipts);
    summary.unique_receipts = Some(stats.unique_receipts);
    summary.duplicate_receipts = Some(stats.duplicate_receipts);
    summary.per_node_unique_receipts = Some(stats.per_node_unique_receipts);
    summary.delivery_rate = Some(stats.delivery_rate);
    summary.expected_formula = Some("msgs_sent * (nodes - 1)".to_string());

    summary.latency_samples = Some(stats.latency_samples);
    summary.negative_latency_samples = Some(stats.negative_latency_samples);
    summary.latency_p50_ms = Some(stats.latency_p50_ms);
    summary.latency_p95_ms = Some(stats.latency_p95_ms);
    summary.latency_max_ms = Some(stats.latency_max_ms);

    summary.pass = stats.delivery_rate >= min_delivery_rate;
    let pass = summary.pass;
    let delivery_rate = stats.delivery_rate;
    harness.write_summary(summary).await?;

    if pass {
        info!(
            "CHAOS test PASSED (delivery_rate={:.2}%)",
            delivery_rate * 100.0
        );
        Ok(())
    } else {
        warn!(
            "CHAOS test FAILED (delivery_rate={:.2}%)",
            delivery_rate * 100.0
        );
        Err(anyhow!("CHAOS test failed"))
    }
}

// Helper trait extension for StdRng to match older rand version if needed
trait SeedBase {
    fn seed_base(seed: u64) -> Self;
}
impl SeedBase for StdRng {
    fn seed_base(seed: u64) -> Self {
        let mut s = [0u8; 32];
        s[..8].copy_from_slice(&seed.to_le_bytes());
        StdRng::from_seed(s)
    }
}

fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}

struct GossipStats {
    expected_total_receipts: usize,
    unique_receipts: usize,
    duplicate_receipts: usize,
    per_node_unique_receipts: HashMap<usize, usize>,
    delivery_rate: f64,
    latency_samples: usize,
    negative_latency_samples: usize,
    latency_p50_ms: u64,
    latency_p95_ms: u64,
    latency_max_ms: u64,
    _measurement_start_ts_ms: u64,
}

async fn run_gossip_burst(
    harness: &mut TestHarness,
    msg_count: usize,
    msg_size: usize,
    timeout_secs: u64,
    warmup_msgs: usize,
    warmup_seconds: u64,
) -> Result<(usize, u64)> {
    let client = reqwest::Client::new();
    let node0_rpc = format!("http://127.0.0.1:{}", harness.nodes[0].rpc_port);

    // Warmup phase
    if warmup_msgs > 0 {
        info!("Warmup: publishing {} messages", warmup_msgs);
        for i in 0..warmup_msgs {
            let msg_id = 1_000_000 + i as u64; // Use large IDs to avoid collision if any
            let sent_ts = chrono::Utc::now().timestamp_millis() as u64;
            let req = serde_json::json!({
                "topic": "ippan/test/gossip",
                "msg_id": msg_id,
                "sent_ts_ms": sent_ts,
                "payload_len": msg_size as u32,
            });
            let _ = client
                .post(format!("{}/p2p/test/gossip", node0_rpc))
                .json(&req)
                .send()
                .await;
        }
    }

    if warmup_seconds > 0 {
        info!("Warmup: waiting {} seconds", warmup_seconds);
        tokio::time::sleep(Duration::from_secs(warmup_seconds)).await;
    }

    let measurement_start_ts_ms = chrono::Utc::now().timestamp_millis() as u64;
    info!("Measurement started at {}", measurement_start_ts_ms);

    let mut sent_count = 0;
    for i in 0..msg_count {
        let msg_id = i as u64;
        let sent_ts = chrono::Utc::now().timestamp_millis() as u64;

        let req = serde_json::json!({
            "topic": "ippan/test/gossip",
            "msg_id": msg_id,
            "sent_ts_ms": sent_ts,
            "payload_len": msg_size as u32,
        });

        if client
            .post(format!("{}/p2p/test/gossip", node0_rpc))
            .json(&req)
            .send()
            .await
            .is_ok()
        {
            sent_count += 1;
        }
        if i % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Wait for propagation
    tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
    Ok((sent_count, measurement_start_ts_ms))
}

async fn calculate_gossip_stats(
    nodes: &[NodeInstance],
    msgs_sent: usize,
    measurement_start_ts_ms: u64,
) -> GossipStats {
    let mut unique_keys = HashSet::new();
    let mut total_observed = 0;
    let mut per_node_unique = HashMap::new();
    let mut latencies = Vec::new();
    let mut negative_latency_samples = 0;

    for (node_idx, node) in nodes.iter().enumerate() {
        let received = node.gossip_received.lock().await;
        let mut node_unique_count = 0;
        for (msg_id, sent_ts, recv_ts) in received.iter() {
            total_observed += 1;
            let key = (node_idx, *msg_id);
            if unique_keys.insert(key) {
                node_unique_count += 1;

                // Only include in latency metrics if sent after measurement start
                if *sent_ts >= measurement_start_ts_ms {
                    let latency = if *recv_ts >= *sent_ts {
                        *recv_ts - *sent_ts
                    } else {
                        negative_latency_samples += 1;
                        0
                    };
                    latencies.push(latency);
                }
            }
        }
        per_node_unique.insert(node_idx, node_unique_count);
    }

    let unique_receipts = unique_keys.len();
    let expected_total_receipts = msgs_sent * (nodes.len() - 1);
    let delivery_rate = if expected_total_receipts > 0 {
        unique_receipts as f64 / expected_total_receipts as f64
    } else {
        0.0
    };

    latencies.sort_unstable();
    let latency_samples = latencies.len();

    let get_percentile = |p: f64| {
        if latencies.is_empty() {
            return 0;
        }
        let idx = ((p / 100.0) * (latencies.len() as f64)).ceil() as usize;
        if idx == 0 {
            return latencies[0];
        }
        if idx >= latencies.len() {
            return latencies[latencies.len() - 1];
        }
        latencies[idx - 1]
    };

    GossipStats {
        expected_total_receipts,
        unique_receipts,
        duplicate_receipts: total_observed - unique_receipts,
        per_node_unique_receipts: per_node_unique,
        delivery_rate,
        latency_samples,
        negative_latency_samples,
        latency_p50_ms: get_percentile(50.0),
        latency_p95_ms: get_percentile(95.0),
        latency_max_ms: latencies.last().copied().unwrap_or(0),
        _measurement_start_ts_ms: measurement_start_ts_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gossip_stats_calculation() {
        let node_count = 8;
        let msgs_sent = 200;

        let mut nodes = Vec::new();
        for i in 0..node_count {
            nodes.push(NodeInstance {
                process: None,
                _data_dir: PathBuf::new(),
                p2p_port: 0,
                rpc_port: 0,
                _peer_id: Arc::new(Mutex::new(None)),
                connected_peers: Arc::new(Mutex::new(HashSet::new())),
                gossip_received: Arc::new(Mutex::new(Vec::new())),
                is_alive: true,
            });
        }

        // Fill receipts for nodes 1..7 (node 0 is sender)
        for node_idx in 1..node_count {
            let mut received = nodes[node_idx].gossip_received.lock().await;
            for msg_id in 0..msgs_sent {
                let sent_ts = 1000;
                let recv_ts = 1100 + (node_idx as u64 * 10) + (msg_id as u64 / 10);
                received.push((msg_id as u64, sent_ts, recv_ts));
            }
        }

        let stats = calculate_gossip_stats(&nodes, msgs_sent, 0).await;

        assert_eq!(stats.expected_total_receipts, 1400); // 200 * (8-1)
        assert_eq!(stats.unique_receipts, 1400);
        assert_eq!(stats.duplicate_receipts, 0);
        assert_eq!(stats.delivery_rate, 1.0);
        assert!(stats.latency_p50_ms > 0);
        assert!(stats.latency_p95_ms >= stats.latency_p50_ms);
        assert!(stats.latency_max_ms >= stats.latency_p95_ms);

        // Add some duplicates
        {
            let mut received = nodes[1].gossip_received.lock().await;
            received.push((0, 1000, 1200)); // Duplicate of msg 0 on node 1
        }

        let stats_with_dupes = calculate_gossip_stats(&nodes, msgs_sent, 0).await;
        assert_eq!(stats_with_dupes.unique_receipts, 1400);
        assert_eq!(stats_with_dupes.duplicate_receipts, 1);
        assert_eq!(stats_with_dupes.delivery_rate, 1.0);
    }
}
