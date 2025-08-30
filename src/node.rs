use crate::block::BlockBuilder;
use crate::crypto::KeyPair;
use crate::error::{Error, Result};
use crate::mempool::Mempool;
use crate::metrics::Metrics;
use crate::network::{NetworkManager, NetworkMessage};
use crate::round::RoundManager;
use crate::state::StateManager;
use crate::time::IppanTime;
use crate::transaction::Transaction;
use crate::wallet::WalletManager;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use axum::serve;
use libp2p::identity;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info};

#[derive(Clone)]
pub struct Node {
    // Core components
    ippan_time: Arc<IppanTime>,
    mempool: Arc<Mempool>,
    state_manager: Arc<StateManager>,
    round_manager: Arc<RoundManager>,
    wallet_manager: Arc<WalletManager>,
    metrics: Arc<Metrics>,
    
    // Network
    network_manager: Arc<RwLock<Option<NetworkManager>>>,
    network_keypair: Arc<identity::Keypair>,
    
    // Block building
    block_builder: Arc<BlockBuilder>,
    builder_keypair: Arc<KeyPair>,
    
    // Configuration
    http_port: u16,
    p2p_port: u16,
    shard_count: usize,
    
    // State
    current_round_id: Arc<RwLock<u64>>,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    status: String,
    peers: usize,
    mempool_size: usize,
    round_id: u64,
    uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionRequest {
    transaction: String, // Hex-encoded transaction
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionResponse {
    success: bool,
    message: String,
    tx_id: Option<String>,
}

impl Node {
    pub async fn new(http_port: u16, p2p_port: u16, shard_count: usize) -> Result<Self> {
        info!("Initializing IPPAN node...");
        
        // Generate keypairs
        let network_keypair = identity::Keypair::generate_ed25519();
        let builder_keypair = KeyPair::generate();
        
        // Create core components
        let ippan_time = Arc::new(IppanTime::new());
        let mempool = Arc::new(Mempool::new(shard_count));
        let state_manager = Arc::new(StateManager::new(10)); // Snapshot every 10 rounds
        let round_manager = Arc::new(RoundManager::new(200, builder_keypair.clone())); // 200ms rounds
        let wallet_manager = Arc::new(WalletManager::new());
        let metrics = Arc::new(Metrics::new(shard_count));
        
        // Create block builder
        let block_builder = Arc::new(BlockBuilder::new());
        
        // Initialize network manager (will be started later)
        let network_manager = Arc::new(RwLock::new(None));
        
        Ok(Self {
            ippan_time,
            mempool,
            state_manager,
            round_manager,
            wallet_manager,
            metrics,
            network_manager,
            network_keypair: Arc::new(network_keypair),
            block_builder,
            builder_keypair: Arc::new(builder_keypair),
            http_port,
            p2p_port,
            shard_count,
            current_round_id: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting IPPAN node...");
        
        // Start network
        self.start_network().await?;
        
        // Start block building loop
        self.start_block_building_loop().await;
        
        // Start round management loop
        self.start_round_management_loop().await;
        
        // Start metrics collection loop
        self.start_metrics_loop().await;
        
        // Start HTTP server
        self.start_http_server().await?;
        
        Ok(())
    }

    async fn start_network(&self) -> Result<()> {
        info!("Starting network manager...");
        
        let network_manager = NetworkManager::new(self.network_keypair.as_ref().clone(), self.metrics.clone()).await?;
        let listen_addr = format!("0.0.0.0:{}", self.p2p_port);
        
        // Store network manager first
        *self.network_manager.write().await = Some(network_manager.clone());
        
        // Start network in background task
        let mut network_manager_clone = network_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = network_manager_clone.start(&listen_addr).await {
                error!("Network manager failed: {}", e);
            }
        });
        
        Ok(())
    }

    async fn start_block_building_loop(&self) {
        let mempool = self.mempool.clone();
        let block_builder = self.block_builder.clone();
        let builder_keypair = self.builder_keypair.clone();
        let ippan_time = self.ippan_time.clone();
        let metrics = self.metrics.clone();
        let current_round_id = self.current_round_id.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(50)); // Build blocks every 50ms
            
            loop {
                interval.tick().await;
                
                let round_id = *current_round_id.read().await;
                let block_time = ippan_time.ippan_time_us().await;
                
                // Get transactions from mempool
                let transactions = mempool.get_transactions_for_block(100).await; // Target 100 txs per block
                
                if !transactions.is_empty() {
                    // Build block
                    let start_time = Instant::now();
                    
                    let block = match block_builder.build_block(
                        vec![], // No parent refs for now
                        round_id,
                        block_time,
                        crate::crypto::hash(&builder_keypair.public_key),
                        transactions.clone(),
                    ) {
                        Ok(block) => block,
                        Err(e) => {
                            error!("Failed to build block: {}", e);
                            continue;
                        }
                    };
                    
                    let duration = start_time.elapsed();
                    let block_size = block.size().unwrap_or(0);
                    
                    // Record metrics
                    metrics.record_block_created(block_size, duration.as_millis() as f64);
                    
                    info!("Built block with {} transactions in {:?}", transactions.len(), duration);
                }
            }
        });
    }

    async fn start_round_management_loop(&self) {
        let round_manager = self.round_manager.clone();
        let current_round_id = self.current_round_id.clone();
        let metrics = self.metrics.clone();
        let state_manager = self.state_manager.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(200)); // 200ms rounds
            
            loop {
                interval.tick().await;
                
                let round_id = *current_round_id.read().await;
                let start_time = Instant::now();
                
                // Start new round
                let verifier_set = vec![crate::crypto::hash(&[0u8; 32])]; // Simplified for now
                round_manager.start_new_round(round_id + 1, start_time.elapsed().as_micros() as u64, verifier_set).await;
                
                // Record metrics
                metrics.record_round_started();
                
                // End current round after delay
                tokio::time::sleep(Duration::from_millis(150)).await;
                
                let end_time = Instant::now();
                let duration = end_time.duration_since(start_time);
                
                round_manager.end_current_round(end_time.elapsed().as_micros() as u64).await;
                
                // Check for finalized rounds
                let finalized_rounds = round_manager.get_finalized_rounds().await;
                for round in finalized_rounds {
                    metrics.record_round_finalized(duration.as_millis() as f64, round.get_transaction_count());
                    
                    // Apply finalized blocks to state
                    let blocks = round.get_finalized_blocks();
                    if !blocks.is_empty() {
                        // This is simplified - in reality you'd need to get the actual transactions
                        let _ = state_manager.apply_finalized_round(&blocks, &[]).await;
                    }
                }
                
                // Update round ID
                *current_round_id.write().await = round_id + 1;
            }
        });
    }

    async fn start_metrics_loop(&self) {
        let mempool = self.mempool.clone();
        let state_manager = self.state_manager.clone();
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Update metrics every 5 seconds
            
            loop {
                interval.tick().await;
                
                // Update mempool metrics
                let mempool_size = mempool.get_total_size().await;
                metrics.update_mempool_size(mempool_size);
                
                let shard_sizes = mempool.get_shard_sizes().await;
                for (i, size) in shard_sizes.iter().enumerate() {
                    metrics.update_mempool_shard_size(i, *size);
                }
                
                // Update state metrics
                let account_count = state_manager.get_account_count().await;
                metrics.update_accounts_total(account_count);
                
                let total_balance = state_manager.get_total_balance().await;
                metrics.update_total_balance(total_balance);
            }
        });
    }

    async fn start_http_server(&self) -> Result<()> {
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/metrics", get(metrics_handler))
            .route("/tx", post(submit_transaction_handler))
            .with_state(Arc::new(self.clone()));
        
        let addr = format!("0.0.0.0:{}", self.http_port);
        info!("Starting HTTP server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        serve(listener, app).await?;
        
        Ok(())
    }

    pub async fn add_bootstrap_peer(&self, peer_addr: &str) -> Result<()> {
        if let Some(network) = self.network_manager.read().await.as_ref() {
            let mut network_clone = network.clone();
            network_clone.add_bootstrap_peer(peer_addr).await
        } else {
            Err(Error::Network("Network not started".to_string()))
        }
    }
}

async fn health_handler(State(node): State<Arc<Node>>) -> Json<HealthResponse> {
    let peers = if let Some(network) = node.network_manager.read().await.as_ref() {
        network.get_peer_count().await
    } else {
        0
    };
    
    let mempool_size = node.mempool.get_total_size().await;
    let round_id = *node.current_round_id.read().await;
    
    Json(HealthResponse {
        status: "healthy".to_string(),
        peers,
        mempool_size,
        round_id,
        uptime_seconds: 0, // TODO: Track uptime
    })
}

async fn metrics_handler(State(node): State<Arc<Node>>) -> (StatusCode, String) {
    let metrics = node.metrics.get_prometheus_metrics();
    (StatusCode::OK, metrics)
}

async fn submit_transaction_handler(
    State(node): State<Arc<Node>>,
    Json(request): Json<SubmitTransactionRequest>,
) -> Json<SubmitTransactionResponse> {
    // Decode transaction from hex
    let tx_data = match hex::decode(&request.transaction) {
        Ok(data) => data,
        Err(e) => {
            return Json(SubmitTransactionResponse {
                success: false,
                message: format!("Invalid hex encoding: {}", e),
                tx_id: None,
            });
        }
    };
    
    // Deserialize transaction
    let transaction = match Transaction::deserialize(&tx_data) {
        Ok(tx) => tx,
        Err(e) => {
            return Json(SubmitTransactionResponse {
                success: false,
                message: format!("Invalid transaction: {}", e),
                tx_id: None,
            });
        }
    };
    
    // Verify transaction
    if let Err(e) = transaction.verify() {
        return Json(SubmitTransactionResponse {
            success: false,
            message: format!("Transaction verification failed: {}", e),
            tx_id: None,
        });
    }
    
    // Add to mempool
    match node.mempool.add_transaction(transaction.clone()).await {
        Ok(true) => {
            let tx_id = transaction.compute_id().unwrap_or([0u8; 32]);
            
            // Publish to network
            if let Some(network) = node.network_manager.read().await.as_ref() {
                let message_sender = network.get_message_sender();
                let _ = message_sender.send(NetworkMessage::Transaction(transaction));
            }
            
            Json(SubmitTransactionResponse {
                success: true,
                message: "Transaction submitted successfully".to_string(),
                tx_id: Some(hex::encode(tx_id)),
            })
        }
        Ok(false) => Json(SubmitTransactionResponse {
            success: false,
            message: "Transaction rejected (mempool full or invalid nonce)".to_string(),
            tx_id: None,
        }),
        Err(e) => Json(SubmitTransactionResponse {
            success: false,
            message: format!("Failed to add transaction to mempool: {}", e),
            tx_id: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_creation() {
        let node = Node::new(8080, 8081, 2).await;
        assert!(node.is_ok());
    }

    #[tokio::test]
    async fn test_health_response() {
        let node = Node::new(8080, 8081, 1).await.unwrap();
        let node_arc = Arc::new(node);
        
        let response = health_handler(State(node_arc)).await;
        assert_eq!(response.status, "healthy");
    }
}
