use crate::{
    api::http::{create_router, AppState},
    p2p::P2PNode,
    mempool::Mempool,
    block::BlockBuilder,
    round::RoundManager,
    state::StateManager,
    metrics::Metrics,
};
use axum::serve;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::net::SocketAddr;
use anyhow::Result;

/// Main IPPAN node orchestrator
pub struct Node {
    p2p: Arc<RwLock<P2PNode>>,
    mempool: Arc<RwLock<Mempool>>,
    block_builder: Arc<RwLock<BlockBuilder>>,
    round_manager: Arc<RwLock<RoundManager>>,
    state_manager: Arc<RwLock<StateManager>>,
    metrics: Arc<Metrics>,
    http_port: u16,
    p2p_port: u16,
}

impl Node {
    pub async fn new(http_port: u16, p2p_port: u16, shards: usize) -> Result<Self> {
        let p2p = Arc::new(RwLock::new(P2PNode::new()?));
        let mempool = Arc::new(RwLock::new(Mempool::new(shards)));
        let block_builder = Arc::new(RwLock::new(BlockBuilder::new()));
        let round_manager = Arc::new(RwLock::new(RoundManager::new()));
        let state_manager = Arc::new(RwLock::new(StateManager::new()));
        let metrics = Arc::new(Metrics::new());

        Ok(Self {
            p2p,
            mempool,
            block_builder,
            round_manager,
            state_manager,
            metrics,
            http_port,
            p2p_port,
        })
    }

    /// Start the IPPAN node
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting IPPAN node...");

        // Start P2P networking
        let p2p_addr = format!("0.0.0.0:{}", self.p2p_port);
        {
            let mut p2p = self.p2p.write().await;
            p2p.start(p2p_addr).await?;
        }

        // Start HTTP server
        self.start_http_server().await?;

        // Start background tasks
        self.start_background_tasks().await?;

        tracing::info!("IPPAN node started successfully");
        tracing::info!("HTTP API: http://localhost:{}", self.http_port);
        tracing::info!("P2P: tcp://0.0.0.0:{}", self.p2p_port);

        Ok(())
    }

    /// Start the HTTP server
    async fn start_http_server(&self) -> Result<()> {
        let app_state = AppState {
            mempool: self.mempool.clone(),
            p2p: self.p2p.clone(),
            metrics: self.metrics.clone(),
        };

        let router = create_router(app_state);
        let addr = SocketAddr::from(([0, 0, 0, 0], self.http_port));

        tracing::info!("Starting HTTP server on {}", addr);

        // Start the server in a separate task
        let server_task = tokio::spawn(async move {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    tracing::info!("HTTP server bound successfully to {}", addr);
                    match serve(listener, router).await {
                        Ok(_) => tracing::info!("HTTP server stopped normally"),
                        Err(e) => tracing::error!("HTTP server error: {}", e),
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to bind HTTP server to {}: {}", addr, e);
                }
            }
        });

        // Wait a bit to ensure the server starts
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// Start background tasks
    async fn start_background_tasks(&self) -> Result<()> {
        // Start block building task
        let block_builder = self.block_builder.clone();
        let mempool = self.mempool.clone();
        let metrics = self.metrics.clone();
        let state_manager = self.state_manager.clone();
        let round_manager = self.round_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
            
            loop {
                interval.tick().await;
                
                let start_time = std::time::Instant::now();
                
                // Get transactions from mempool
                let transactions = {
                    let mempool = mempool.read().await;
                    mempool.get_transactions_for_block(1000).await
                };

                if !transactions.is_empty() {
                    // Build block
                    let block_id = {
                        let mut builder = block_builder.write().await;
                        builder.build_block(&transactions).await
                    };

                    if let Ok(block_id) = block_id {
                        metrics.record_block_built();
                        metrics.record_block_build_time(start_time.elapsed());
                        
                        // Add block to current round
                        {
                            let mut round_mgr = round_manager.write().await;
                            if let Err(e) = round_mgr.add_block(block_id).await {
                                tracing::warn!("Failed to add block to round: {}", e);
                            }
                        }
                        
                        // Apply transactions to state
                        let mut state = state_manager.write().await;
                        for tx in transactions {
                            if let Err(e) = state.apply_transaction(&tx).await {
                                tracing::warn!("Failed to apply transaction: {}", e);
                            } else {
                                metrics.record_transaction_finalized();
                            }
                        }
                    }
                }
            }
        });

        // Start round management task
        let round_manager = self.round_manager.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
            
            loop {
                interval.tick().await;
                
                let start_time = std::time::Instant::now();
                
                let round_id = {
                    let mut manager = round_manager.write().await;
                    manager.start_round().await
                };

                if let Ok(_round_id) = round_id {
                    metrics.record_round_completed();
                    metrics.record_round_duration(start_time.elapsed());
                    tracing::debug!("Started new round");
                }
            }
        });

        // Start metrics update task
        let mempool = self.mempool.clone();
        let p2p = self.p2p.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Update mempool size
                let mempool_size = {
                    let mempool = mempool.read().await;
                    mempool.size()
                };
                metrics.update_mempool_size(mempool_size as u64);
                
                // Update peer count
                let peer_count = {
                    let p2p = p2p.read().await;
                    p2p.peer_count()
                };
                metrics.update_active_peers(peer_count as u64);
            }
        });

        // Start mempool cleanup task
        let mempool = self.mempool.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut mempool = mempool.write().await;
                mempool.cleanup().await;
            }
        });

        // Start state snapshot task
        let state_manager = self.state_manager.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let mut state = state_manager.write().await;
                if let Err(e) = state.take_snapshot().await {
                    tracing::warn!("Failed to take state snapshot: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Add a bootstrap peer
    pub async fn add_bootstrap_peer(&mut self, peer: &str) -> Result<()> {
        let mut p2p = self.p2p.write().await;
        p2p.add_bootstrap_peer(peer).await?;
        tracing::info!("Added bootstrap peer: {}", peer);
        Ok(())
    }

    /// Get node statistics
    pub async fn get_stats(&self) -> NodeStats {
        let mempool_stats = {
            let mempool = self.mempool.read().await;
            mempool.get_stats()
        };

        let peer_count = {
            let p2p = self.p2p.read().await;
            p2p.peer_count()
        };

        let current_tps = self.metrics.get_current_tps();

        NodeStats {
            mempool: mempool_stats,
            peer_count,
            current_tps,
            uptime_seconds: 0, // TODO: Add uptime tracking
        }
    }
}

#[derive(Debug)]
pub struct NodeStats {
    pub mempool: crate::mempool::MempoolStats,
    pub peer_count: usize,
    pub current_tps: f64,
    pub uptime_seconds: u64,
}
