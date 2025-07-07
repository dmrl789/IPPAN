use crate::{
    config::Config,
    consensus::ConsensusEngine,
    dht::DhtNode,
    error::{IppanError, Result},
    network::NetworkManager,
    storage::StorageManager,
    staking::StakingManager,
    wallet::WalletManager,
    api::ApiServer,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Main IPPAN node that orchestrates all components
pub struct IppanNode {
    config: Config,
    consensus: Arc<RwLock<ConsensusEngine>>,
    network: Arc<NetworkManager>,
    storage: Arc<StorageManager>,
    dht: Arc<DhtNode>,
    staking: Arc<StakingManager>,
    wallet: Arc<WalletManager>,
    api: Option<ApiServer>,
    running: bool,
}

impl IppanNode {
    /// Create a new IPPAN node
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing IPPAN node...");
        
        // Initialize storage manager
        let storage = Arc::new(StorageManager::new(config.storage.clone()).await?);
        info!("Storage manager initialized");
        
        // Initialize wallet manager
        let wallet = Arc::new(WalletManager::new().await?);
        info!("Wallet manager initialized");
        
        // Initialize staking manager
        let staking = Arc::new(StakingManager::new(
            config.staking.clone(),
            wallet.clone(),
        ).await?);
        info!("Staking manager initialized");
        
        // Initialize DHT node
        let dht = Arc::new(DhtNode::new(
            config.dht.clone(),
            storage.clone(),
        ).await?);
        info!("DHT node initialized");
        
        // Initialize network manager
        let network = Arc::new(NetworkManager::new(
            config.network.clone(),
            dht.clone(),
        ).await?);
        info!("Network manager initialized");
        
        // Initialize consensus engine
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(
            config.consensus.clone(),
        ).await?));
        info!("Consensus engine initialized");
        
        // Initialize API server if enabled
        let api = if config.api.listen_addr != "disabled" {
            Some(ApiServer::new(
                config.api.clone(),
            ).await?)
        } else {
            None
        };
        
        if api.is_some() {
            info!("API server initialized");
        }
        
        info!("IPPAN node initialization complete");
        
        Ok(Self {
            config,
            consensus,
            network,
            storage,
            dht,
            staking,
            wallet,
            api,
            running: false,
        })
    }
    
    /// Start the IPPAN node
    pub async fn start(&mut self) -> Result<()> {
        if self.running {
            warn!("IPPAN node is already running");
            return Ok(());
        }
        
        info!("Starting IPPAN node...");
        
        // Start network manager
        self.network.start().await?;
        info!("Network manager started");
        
        // Start consensus engine
        {
            let mut consensus = self.consensus.write().await;
            consensus.start().await?;
        }
        info!("Consensus engine started");
        
        // Start DHT node
        self.dht.start().await?;
        info!("DHT node started");
        
        // Start storage manager
        self.storage.start().await?;
        info!("Storage manager started");
        
        // Start staking manager
        self.staking.start().await?;
        info!("Staking manager started");
        
        // Start API server if enabled
        if let Some(ref mut api) = self.api {
            api.start().await?;
            info!("API server started");
        }
        
        self.running = true;
        info!("IPPAN node started successfully");
        
        // Keep the node running
        self.run_event_loop().await?;
        
        Ok(())
    }
    
    /// Stop the IPPAN node
    pub async fn stop(&mut self) -> Result<()> {
        if !self.running {
            warn!("IPPAN node is not running");
            return Ok(());
        }
        
        info!("Stopping IPPAN node...");
        
        // Stop API server
        if let Some(ref mut api) = self.api {
            api.stop().await?;
            info!("API server stopped");
        }
        
        // Stop consensus engine
        {
            let mut consensus = self.consensus.write().await;
            consensus.stop().await?;
        }
        info!("Consensus engine stopped");
        
        // Stop network manager
        self.network.stop().await?;
        info!("Network manager stopped");
        
        // Stop DHT node
        self.dht.stop().await?;
        info!("DHT node stopped");
        
        // Stop storage manager
        self.storage.stop().await?;
        info!("Storage manager stopped");
        
        // Stop staking manager
        self.staking.stop().await?;
        info!("Staking manager stopped");
        
        self.running = false;
        info!("IPPAN node stopped successfully");
        
        Ok(())
    }
    
    /// Main event loop for the node
    async fn run_event_loop(&self) -> Result<()> {
        info!("Entering main event loop");
        
        // Set up shutdown signal handler
        let mut shutdown_signal = tokio::signal::ctrl_c();
        
        loop {
            tokio::select! {
                _ = &mut shutdown_signal => {
                    info!("Received shutdown signal");
                    break;
                }
                
                // Handle consensus events
                consensus_event = self.handle_consensus_events() => {
                    if let Err(e) = consensus_event {
                        error!("Consensus event error: {}", e);
                    }
                }
                
                // Handle network events
                network_event = self.handle_network_events() => {
                    if let Err(e) = network_event {
                        error!("Network event error: {}", e);
                    }
                }
                
                // Handle storage events
                storage_event = self.handle_storage_events() => {
                    if let Err(e) = storage_event {
                        error!("Storage event error: {}", e);
                    }
                }
                
                // Handle DHT events
                dht_event = self.handle_dht_events() => {
                    if let Err(e) = dht_event {
                        error!("DHT event error: {}", e);
                    }
                }
            }
        }
        
        info!("Exiting main event loop");
        Ok(())
    }
    
    /// Handle consensus events
    async fn handle_consensus_events(&self) -> Result<()> {
        let consensus = self.consensus.read().await;
        
        // Process new blocks
        if let Some(block) = consensus.get_next_block().await? {
            info!("Processing new block: {:?}", block.hash());
            
            // Validate block
            consensus.validate_block(&block).await?;
            
            // Add block to DAG
            consensus.add_block(block).await?;
            
            // Update staking state
            self.staking.update_for_block(&block).await?;
        }
        
        // Process new transactions
        while let Some(tx) = consensus.get_next_transaction().await? {
            info!("Processing new transaction: {:?}", tx.hash());
            
            // Validate transaction
            consensus.validate_transaction(&tx).await?;
            
            // Add transaction to mempool
            consensus.add_transaction(tx).await?;
        }
        
        Ok(())
    }
    
    /// Handle network events
    async fn handle_network_events(&self) -> Result<()> {
        // Process incoming messages
        while let Some(message) = self.network.receive_message().await? {
            match message {
                crate::network::Message::NewBlock(block) => {
                    let mut consensus = self.consensus.write().await;
                    consensus.handle_new_block(block).await?;
                }
                crate::network::Message::NewTransaction(tx) => {
                    let mut consensus = self.consensus.write().await;
                    consensus.handle_new_transaction(tx).await?;
                }
                crate::network::Message::StorageRequest(request) => {
                    self.storage.handle_request(request).await?;
                }
                crate::network::Message::DhtLookup(lookup) => {
                    self.dht.handle_lookup(lookup).await?;
                }
                _ => {
                    warn!("Unhandled network message: {:?}", message);
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle storage events
    async fn handle_storage_events(&self) -> Result<()> {
        // Process storage proofs
        while let Some(proof) = self.storage.get_next_proof().await? {
            info!("Processing storage proof: {:?}", proof.id());
            
            // Validate proof
            self.storage.validate_proof(&proof).await?;
            
            // Update staking rewards
            self.staking.update_for_storage_proof(&proof).await?;
        }
        
        // Process storage requests
        while let Some(request) = self.storage.get_next_request().await? {
            info!("Processing storage request: {:?}", request.id());
            
            // Handle request
            self.storage.handle_request(request).await?;
        }
        
        Ok(())
    }
    
    /// Handle DHT events
    async fn handle_dht_events(&self) -> Result<()> {
        // Process DHT lookups
        while let Some(lookup) = self.dht.get_next_lookup().await? {
            info!("Processing DHT lookup: {:?}", lookup.key());
            
            // Handle lookup
            self.dht.handle_lookup(lookup).await?;
        }
        
        // Process DHT storage operations
        while let Some(operation) = self.dht.get_next_operation().await? {
            info!("Processing DHT operation: {:?}", operation);
            
            // Handle operation
            self.dht.handle_operation(operation).await?;
        }
        
        Ok(())
    }
    
    /// Get node status
    pub async fn status(&self) -> NodeStatus {
        let consensus = self.consensus.read().await;
        
        NodeStatus {
            running: self.running,
            node_id: self.network.node_id(),
            current_block: consensus.current_block_number(),
            current_round: consensus.current_round(),
            connected_peers: self.network.connected_peers_count(),
            storage_used: self.storage.used_space(),
            storage_total: self.storage.total_space(),
            stake_amount: self.staking.stake_amount(),
            wallet_balance: self.wallet.balance(),
        }
    }
}

/// Node status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeStatus {
    pub running: bool,
    pub node_id: String,
    pub current_block: u64,
    pub current_round: u64,
    pub connected_peers: usize,
    pub storage_used: u64,
    pub storage_total: u64,
    pub stake_amount: u64,
    pub wallet_balance: u64,
}

impl Drop for IppanNode {
    fn drop(&mut self) {
        if self.running {
            // Try to stop the node gracefully
            let runtime = tokio::runtime::Handle::current();
            if let Ok(()) = runtime.block_on(self.stop()) {
                info!("IPPAN node stopped gracefully during drop");
            } else {
                error!("Failed to stop IPPAN node gracefully during drop");
            }
        }
    }
} 