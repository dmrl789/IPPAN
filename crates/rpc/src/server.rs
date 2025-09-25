use anyhow::Result;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use ippan_storage::Storage;
use ippan_p2p::HttpP2PNetwork;
use ippan_types::Transaction;

#[derive(Debug, Clone, Default)]
pub struct L2Config {
    pub max_commit_size: usize,
    pub min_epoch_gap_ms: u64,
    pub challenge_window_ms: u64,
    pub da_mode: String,
    pub max_l2_count: usize,
}

#[derive(Clone)]
pub struct ConsensusHandle {
    // minimal placeholder; extend with real types as needed
    pub tx_sender: mpsc::UnboundedSender<Transaction>,
}

impl ConsensusHandle {
    pub fn new<CS: Send + Sync + 'static>(
        _consensus: Arc<tokio::sync::Mutex<CS>>,
        tx_sender: mpsc::UnboundedSender<Transaction>,
        _mempool: Arc<parking_lot::RwLock<Vec<Transaction>>>,
    ) -> Self {
        Self { tx_sender }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub start_time: Instant,
    pub peer_count: Arc<AtomicUsize>,
    pub p2p_network: Option<Arc<HttpP2PNetwork>>,
    pub tx_sender: Option<mpsc::UnboundedSender<Transaction>>,
    pub node_id: String,
    pub consensus: Option<ConsensusHandle>,
    pub l2_config: L2Config,
}

pub async fn start_server(_state: AppState, addr: &str) -> Result<()> {
    let app = Router::new().route("/health", get(|| async { "ok" }));

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}


