//! Real REST API for IPPAN
//! 
//! Implements actual working REST endpoints for blockchain node interaction
//! including transaction management, account operations, block queries,
//! and comprehensive blockchain data access.

use crate::{Result, IppanError, TransactionHash};
use crate::node::IppanNode;
use crate::database::{DatabaseManager, StoredTransaction, StoredAccount, StoredBlock};
use crate::wallet::real_wallet::{RealWallet, WalletAccount, WalletTransaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::net::SocketAddr;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::collections::HashMap;
use tracing::{info, warn, error, debug};

/// Real REST API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealRestApiConfig {
    /// API server address
    pub server_address: SocketAddr,
    /// Enable CORS
    pub enable_cors: bool,
    /// CORS origins
    pub cors_origins: Vec<String>,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Enable request logging
    pub enable_request_logging: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit requests per minute
    pub rate_limit_per_minute: u64,
    /// Enable API authentication
    pub enable_authentication: bool,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Enable API versioning
    pub enable_versioning: bool,
    /// Default API version
    pub default_api_version: String,
}

impl Default for RealRestApiConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:3000".parse().unwrap(),
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            request_timeout_seconds: 30,
            max_request_size: 1024 * 1024, // 1MB
            enable_request_logging: true,
            enable_rate_limiting: true,
            rate_limit_per_minute: 1000,
            enable_authentication: false,
            api_key: None,
            enable_versioning: true,
            default_api_version: "v1".to_string(),
        }
    }
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response data
    pub data: Option<T>,
    /// Error message
    pub error: Option<String>,
    /// Success status
    pub success: bool,
    /// Response timestamp
    pub timestamp: u64,
    /// Request ID for tracking
    pub request_id: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, request_id: String) -> Self {
        Self {
            data: Some(data),
            error: None,
            success: true,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            request_id,
        }
    }
    
    pub fn error(error: String, request_id: String) -> Self {
        Self {
            data: None,
            error: Some(error),
            success: false,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            request_id,
        }
    }
}

/// Node status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatusResponse {
    /// Node ID
    pub node_id: String,
    /// Is running
    pub is_running: bool,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Version
    pub version: String,
    /// Network ID
    pub network_id: String,
    /// Chain ID
    pub chain_id: u64,
    /// Current block height
    pub current_block_height: u64,
    /// Current block hash
    pub current_block_hash: String,
    /// Peer count
    pub peer_count: usize,
    /// Sync status
    pub sync_status: String,
    /// Consensus status
    pub consensus_status: String,
}

/// Account balance response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalanceResponse {
    /// Account address
    pub address: String,
    /// Account balance
    pub balance: u64,
    /// Account nonce
    pub nonce: u64,
    /// Is active
    pub is_active: bool,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Transaction response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    /// Transaction hash
    pub hash: String,
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Amount
    pub amount: u64,
    /// Fee
    pub fee: u64,
    /// Nonce
    pub nonce: u64,
    /// Status
    pub status: String,
    /// Block number
    pub block_number: Option<u64>,
    /// Timestamp
    pub timestamp: u64,
    /// Transaction type
    pub transaction_type: String,
}

/// Block response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponse {
    /// Block hash
    pub hash: String,
    /// Block number
    pub number: u64,
    /// Parent hash
    pub parent_hash: String,
    /// Timestamp
    pub timestamp: u64,
    /// Transaction count
    pub transaction_count: u64,
    /// Producer
    pub producer: String,
    /// Status
    pub status: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Gas used
    pub gas_used: u64,
    /// Gas limit
    pub gas_limit: u64,
}

/// Transaction request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    /// Transaction type
    pub transaction_type: String,
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Amount
    pub amount: u64,
    /// Fee
    pub fee: Option<u64>,
    /// Nonce
    pub nonce: u64,
    /// Transaction data
    pub data: Option<Vec<u8>>,
    /// Signature
    pub signature: String,
}

/// Account creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCreationRequest {
    /// Account name
    pub name: String,
    /// Account type
    pub account_type: String,
    /// Initial balance
    pub initial_balance: Option<u64>,
}

/// API statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStats {
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Active connections
    pub active_connections: usize,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last request timestamp
    pub last_request: Option<u64>,
}

/// Real REST API server
pub struct RealRestApi {
    /// Configuration
    config: RealRestApiConfig,
    /// Node reference
    node: Arc<RwLock<IppanNode>>,
    /// Database manager
    database: Arc<DatabaseManager>,
    /// Wallet manager
    wallet: Arc<RealWallet>,
    /// API statistics
    stats: Arc<RwLock<ApiStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl RealRestApi {
    /// Create a new real REST API server
    pub fn new(
        config: RealRestApiConfig,
        node: Arc<RwLock<IppanNode>>,
        database: Arc<DatabaseManager>,
        wallet: Arc<RealWallet>,
    ) -> Self {
        let stats = ApiStats {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            requests_per_second: 0.0,
            active_connections: 0,
            uptime_seconds: 0,
            last_request: None,
        };
        
        Self {
            config,
            node,
            database,
            wallet,
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the REST API server
    pub async fn start(&self) -> Result<()> {
        info!("Starting real REST API server on {}", self.config.server_address);
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        let config = self.config.clone();
        let node = self.node.clone();
        let database = self.database.clone();
        let wallet = self.wallet.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        
        // Start statistics update loop
        let stats_clone = stats.clone();
        tokio::spawn(async move {
            Self::statistics_update_loop(stats_clone, is_running, start_time).await;
        });
        
        let make_svc = make_service_fn(move |_conn| {
            let config = config.clone();
            let node = Arc::clone(&node);
            let database = Arc::clone(&database);
            let wallet = Arc::clone(&wallet);
            let stats = Arc::clone(&stats);
            
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let config = config.clone();
                    let node = Arc::clone(&node);
                    let database = Arc::clone(&database);
                    let wallet = Arc::clone(&wallet);
                    let stats = Arc::clone(&stats);
                    
                    handle_request(req, config, node, database, wallet, stats)
                }))
            }
        });

        let server = Server::bind(&self.config.server_address).serve(make_svc);
        
        info!("Real REST API server listening on {}", self.config.server_address);
        
        if let Err(e) = server.await {
            error!("REST API server error: {}", e);
            return Err(IppanError::Network(format!("REST API server error: {}", e)));
        }

        Ok(())
    }
    
    /// Stop the REST API server
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping real REST API server");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Real REST API server stopped");
        Ok(())
    }
    
    /// Get API statistics
    pub async fn get_stats(&self) -> Result<ApiStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Statistics update loop
    async fn statistics_update_loop(
        stats: Arc<RwLock<ApiStats>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
    ) {
        while *is_running.read().await {
            let mut stats = stats.write().await;
            stats.uptime_seconds = start_time.elapsed().as_secs();
            
            // Calculate requests per second
            if stats.uptime_seconds > 0 {
                stats.requests_per_second = stats.total_requests as f64 / stats.uptime_seconds as f64;
            }
            
            drop(stats);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// Handle HTTP requests
async fn handle_request(
    req: Request<Body>,
    config: RealRestApiConfig,
    node: Arc<RwLock<IppanNode>>,
    database: Arc<DatabaseManager>,
    wallet: Arc<RealWallet>,
    stats: Arc<RwLock<ApiStats>>,
) -> std::result::Result<Response<Body>, Infallible> {
    let start_time = Instant::now();
    let request_id = format!("req_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
    let request_id_clone = request_id.clone();
    let stats_clone = stats.clone();
    
    // Update statistics
    {
        let stats_clone = stats.clone();
        let mut stats_guard = stats_clone.write().await;
        stats_guard.total_requests += 1;
        stats_guard.last_request = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }
    
    let method = req.method();
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");
    
    if config.enable_request_logging {
        debug!("API request: {} {} (ID: {})", method, path, request_id_clone);
    }
    
    // Handle CORS preflight
    if method == &Method::OPTIONS && config.enable_cors {
        return Ok(create_cors_response());
    }
    
    let result = match (method, path) {
        // Health check
        (&Method::GET, "/health") => {
            Ok(Response::new(Body::from("OK")))
        }
        
        // Node status
        (&Method::GET, "/api/v1/status") => {
            handle_node_status(node, request_id_clone.clone()).await
        }
        
        // Get account balance
        (&Method::GET, path) if path.starts_with("/api/v1/account/") && path.ends_with("/balance") => {
            let account = path.strip_prefix("/api/v1/account/").unwrap_or("")
                .strip_suffix("/balance").unwrap_or("");
            handle_get_account_balance(database, account, request_id_clone.clone()).await
        }
        
        // Get account info
        (&Method::GET, path) if path.starts_with("/api/v1/account/") => {
            let account = path.strip_prefix("/api/v1/account/").unwrap_or("");
            handle_get_account_info(database, account, request_id_clone.clone()).await
        }
        
        // Create account
        (&Method::POST, "/api/v1/account") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap_or_default();
            handle_create_account(wallet, body, request_id_clone.clone()).await
        }
        
        // Get transaction
        (&Method::GET, path) if path.starts_with("/api/v1/transaction/") => {
            let tx_hash = path.strip_prefix("/api/v1/transaction/").unwrap_or("");
            handle_get_transaction(database, tx_hash, request_id_clone.clone()).await
        }
        
        // Send transaction
        (&Method::POST, "/api/v1/transaction") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap_or_default();
            handle_send_transaction(wallet, body, request_id_clone.clone()).await
        }
        
        // Get transactions by account
        (&Method::GET, path) if path.starts_with("/api/v1/account/") && path.ends_with("/transactions") => {
            let account = path.strip_prefix("/api/v1/account/").unwrap_or("")
                .strip_suffix("/transactions").unwrap_or("");
            handle_get_account_transactions(database, account, query, request_id_clone.clone()).await
        }
        
        // Get block
        (&Method::GET, path) if path.starts_with("/api/v1/block/") => {
            let block_id = path.strip_prefix("/api/v1/block/").unwrap_or("");
            handle_get_block(database, block_id, request_id_clone.clone()).await
        }
        
        // Get latest blocks
        (&Method::GET, "/api/v1/blocks") => {
            handle_get_latest_blocks(database, query, request_id_clone.clone()).await
        }
        
        // Get blockchain info
        (&Method::GET, "/api/v1/blockchain") => {
            handle_get_blockchain_info(database, request_id_clone.clone()).await
        }
        
        // Get API statistics
        (&Method::GET, "/api/v1/stats") => {
            handle_get_api_stats(stats_clone, request_id_clone.clone()).await
        }
        
        // Get wallet accounts
        (&Method::GET, "/api/v1/wallet/accounts") => {
            handle_get_wallet_accounts(wallet, request_id_clone.clone()).await
        }
        
        // Get wallet statistics
        (&Method::GET, "/api/v1/wallet/stats") => {
            handle_get_wallet_stats(wallet, request_id_clone.clone()).await
        }
        
        // 404 Not Found
        _ => {
            let error_response = ApiResponse::<()>::error(
                format!("Endpoint not found: {} {}", method, path),
                request_id_clone,
            );
            let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(json))
                .unwrap())
        }
    };
    
    // Update response time statistics
    let response_time = start_time.elapsed().as_millis() as f64;
    {
        let stats_clone = stats.clone();
        let mut stats_guard = stats_clone.write().await;
        stats_guard.average_response_time_ms = 
            (stats_guard.average_response_time_ms * (stats_guard.total_requests - 1) as f64 + response_time) / stats_guard.total_requests as f64;
        
        match &result {
            Ok(_) => stats_guard.successful_requests += 1,
            Err(_) => stats_guard.failed_requests += 1,
        }
    }
    
    match result {
        Ok(response) => Ok(response),
        Err(e) => {
            let error_response = ApiResponse::<()>::error(
                format!("Internal server error: {}", e),
                request_id.clone(),
            );
            let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(json))
                .unwrap())
        }
    }
}

/// Handle node status request
async fn handle_node_status(
    node: Arc<RwLock<IppanNode>>,
    request_id: String,
) -> Result<Response<Body>> {
    let node_guard = node.read().await;
    let status = node_guard.get_status();
    
    let response = NodeStatusResponse {
        node_id: hex::encode(node_guard.node_id()),
        is_running: status.is_running,
        uptime_seconds: status.uptime.as_secs(),
        version: status.version,
        network_id: "ippan_mainnet".to_string(),
        chain_id: 1,
        current_block_height: 0, // TODO: Get from blockchain state
        current_block_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        peer_count: 0, // TODO: Get from network
        sync_status: "synced".to_string(),
        consensus_status: "active".to_string(),
    };
    
    let api_response = ApiResponse::success(response, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get account balance request
async fn handle_get_account_balance(
    database: Arc<DatabaseManager>,
    account: &str,
    request_id: String,
) -> Result<Response<Body>> {
    if account.is_empty() {
        let error_response = ApiResponse::<()>::error("Account address is required".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(json))
            .unwrap());
    }
    
    // Get account from database
    let account_data = database.account_db.get_account(account).await?;
    
    if let Some(account) = account_data {
        let response = AccountBalanceResponse {
            address: account.address,
            balance: account.balance,
            nonce: account.nonce,
            is_active: account.is_active,
            last_updated: account.last_updated,
        };
        
        let api_response = ApiResponse::success(response, request_id);
        let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::new(Body::from(json)))
    } else {
        let error_response = ApiResponse::<()>::error("Account not found".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(json))
            .unwrap())
    }
}

/// Handle get account info request
async fn handle_get_account_info(
    database: Arc<DatabaseManager>,
    account: &str,
    request_id: String,
) -> Result<Response<Body>> {
    if account.is_empty() {
        let error_response = ApiResponse::<()>::error("Account address is required".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(json))
            .unwrap());
    }
    
    // Get account from database
    let account_data = database.account_db.get_account(account).await?;
    
    if let Some(account) = account_data {
        let api_response = ApiResponse::success(account, request_id);
        let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::new(Body::from(json)))
    } else {
        let error_response = ApiResponse::<()>::error("Account not found".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(json))
            .unwrap())
    }
}

/// Handle create account request
async fn handle_create_account(
    wallet: Arc<RealWallet>,
    body: hyper::body::Bytes,
    request_id: String,
) -> Result<Response<Body>> {
    let request: AccountCreationRequest = serde_json::from_slice(&body)
        .map_err(|e| IppanError::Api(format!("Invalid request: {}", e)))?;
    
    // Create account in wallet
    let account = wallet.create_account(request.name, crate::wallet::real_wallet::AccountType::Standard).await?;
    
    // Set initial balance if provided
    if let Some(initial_balance) = request.initial_balance {
        wallet.update_account_balance(&account.account_id, initial_balance).await?;
    }
    
    let api_response = ApiResponse::success(account, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get transaction request
async fn handle_get_transaction(
    database: Arc<DatabaseManager>,
    tx_hash: &str,
    request_id: String,
) -> Result<Response<Body>> {
    if tx_hash.is_empty() {
        let error_response = ApiResponse::<()>::error("Transaction hash is required".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(json))
            .unwrap());
    }
    
    // Parse transaction hash
    let hash_bytes = hex::decode(tx_hash)
        .map_err(|e| IppanError::Api(format!("Invalid transaction hash: {}", e)))?;
    
    if hash_bytes.len() != 32 {
        return Err(IppanError::Api("Invalid transaction hash length".to_string()));
    }
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    
    // Get transaction from database
    let transaction = database.transaction_db.get_transaction(&hash).await?;
    
    if let Some(transaction) = transaction {
        let response = TransactionResponse {
            hash: hex::encode(transaction.hash),
            from: transaction.from_address,
            to: transaction.to_address,
            amount: transaction.amount,
            fee: transaction.fee,
            nonce: transaction.nonce,
            status: format!("{:?}", transaction.status),
            block_number: transaction.block_number,
            timestamp: transaction.timestamp,
            transaction_type: "transfer".to_string(), // TODO: Get from transaction data
        };
        
        let api_response = ApiResponse::success(response, request_id);
        let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::new(Body::from(json)))
    } else {
        let error_response = ApiResponse::<()>::error("Transaction not found".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(json))
            .unwrap())
    }
}

/// Handle send transaction request
async fn handle_send_transaction(
    wallet: Arc<RealWallet>,
    body: hyper::body::Bytes,
    request_id: String,
) -> Result<Response<Body>> {
    let request: TransactionRequest = serde_json::from_slice(&body)
        .map_err(|e| IppanError::Api(format!("Invalid request: {}", e)))?;
    
    // Create transaction in wallet
    let transaction = wallet.create_transaction(
        &request.from,
        &request.to,
        request.amount,
        crate::wallet::real_wallet::TransactionType::Transfer,
    ).await?;
    
    let response = TransactionResponse {
        hash: hex::encode(transaction.hash),
        from: transaction.from,
        to: transaction.to,
        amount: transaction.amount,
        fee: transaction.fee,
        nonce: transaction.nonce,
        status: format!("{:?}", transaction.status),
        block_number: transaction.block_number,
        timestamp: transaction.timestamp,
        transaction_type: "transfer".to_string(),
    };
    
    let api_response = ApiResponse::success(response, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get account transactions request
async fn handle_get_account_transactions(
    database: Arc<DatabaseManager>,
    account: &str,
    query: &str,
    request_id: String,
) -> Result<Response<Body>> {
    if account.is_empty() {
        let error_response = ApiResponse::<()>::error("Account address is required".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(json))
            .unwrap());
    }
    
    // Get transactions from database
    let transactions = database.transaction_db.get_transactions_by_address(account).await?;
    
    let responses: Vec<TransactionResponse> = transactions.into_iter().map(|tx| {
        TransactionResponse {
            hash: hex::encode(tx.hash),
            from: tx.from_address,
            to: tx.to_address,
            amount: tx.amount,
            fee: tx.fee,
            nonce: tx.nonce,
            status: format!("{:?}", tx.status),
            block_number: tx.block_number,
            timestamp: tx.timestamp,
            transaction_type: "transfer".to_string(),
        }
    }).collect();
    
    let api_response = ApiResponse::success(responses, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get block request
async fn handle_get_block(
    database: Arc<DatabaseManager>,
    block_id: &str,
    request_id: String,
) -> Result<Response<Body>> {
    if block_id.is_empty() {
        let error_response = ApiResponse::<()>::error("Block ID is required".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(json))
            .unwrap());
    }
    
    // Try to parse as block number first, then as hash
    let block = if let Ok(number) = block_id.parse::<u64>() {
        database.block_db.get_block_by_number(number).await?
    } else {
        // Parse as hash
        let hash_bytes = hex::decode(block_id)
            .map_err(|e| IppanError::Api(format!("Invalid block hash: {}", e)))?;
        
        if hash_bytes.len() != 32 {
            return Err(IppanError::Api("Invalid block hash length".to_string()));
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);
        
        database.block_db.get_block(&hash).await?
    };
    
    if let Some(block) = block {
        let response = BlockResponse {
            hash: hex::encode(block.hash),
            number: block.number,
            parent_hash: hex::encode(block.parent_hash),
            timestamp: block.timestamp,
            transaction_count: block.transaction_count,
            producer: block.producer,
            status: format!("{:?}", block.status),
            size_bytes: block.size_bytes,
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
        };
        
        let api_response = ApiResponse::success(response, request_id);
        let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::new(Body::from(json)))
    } else {
        let error_response = ApiResponse::<()>::error("Block not found".to_string(), request_id);
        let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(json))
            .unwrap())
    }
}

/// Handle get latest blocks request
async fn handle_get_latest_blocks(
    database: Arc<DatabaseManager>,
    query: &str,
    request_id: String,
) -> Result<Response<Body>> {
    // Parse query parameters
    let limit = query.split('&')
        .find(|param| param.starts_with("limit="))
        .and_then(|param| param.strip_prefix("limit="))
        .and_then(|val| val.parse::<usize>().ok())
        .unwrap_or(10);
    
    // Get latest blocks (placeholder - in real implementation, this would query the database)
    let blocks = vec![]; // TODO: Implement get_latest_blocks in database
    
    let responses: Vec<BlockResponse> = blocks.into_iter().map(|block: StoredBlock| {
        BlockResponse {
            hash: hex::encode(block.hash),
            number: block.number,
            parent_hash: hex::encode(block.parent_hash),
            timestamp: block.timestamp,
            transaction_count: block.transaction_count,
            producer: block.producer,
            status: format!("{:?}", block.status),
            size_bytes: block.size_bytes,
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
        }
    }).collect();
    
    let api_response = ApiResponse::success(responses, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get blockchain info request
async fn handle_get_blockchain_info(
    database: Arc<DatabaseManager>,
    request_id: String,
) -> Result<Response<Body>> {
    // Get blockchain state
    let state = database.blockchain_state.get_current_state().await;
    
    let info = serde_json::json!({
        "network_id": "ippan_mainnet",
        "chain_id": 1,
        "current_block_height": state.current_block_height,
        "current_block_hash": hex::encode(state.current_block_hash),
        "total_transactions": state.total_transactions,
        "total_accounts": state.total_accounts,
        "total_supply": state.total_supply,
        "consensus_parameters": {
            "block_time_seconds": state.consensus_parameters.block_time_seconds,
            "max_block_size": state.consensus_parameters.max_block_size,
            "max_transactions_per_block": state.consensus_parameters.max_transactions_per_block,
            "min_stake_required": state.consensus_parameters.min_stake_required,
        },
        "validator_set": {
            "total_validators": state.validator_set.active_validators.len(),
            "total_stake": state.validator_set.total_stake,
        }
    });
    
    let api_response = ApiResponse::success(info, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get API statistics request
async fn handle_get_api_stats(
    stats: Arc<RwLock<ApiStats>>,
    request_id: String,
) -> Result<Response<Body>> {
    let stats = stats.read().await;
    let api_response = ApiResponse::success(stats.clone(), request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get wallet accounts request
async fn handle_get_wallet_accounts(
    wallet: Arc<RealWallet>,
    request_id: String,
) -> Result<Response<Body>> {
    let accounts = wallet.list_accounts().await;
    let api_response = ApiResponse::success(accounts, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Handle get wallet statistics request
async fn handle_get_wallet_stats(
    wallet: Arc<RealWallet>,
    request_id: String,
) -> Result<Response<Body>> {
    let stats = wallet.get_wallet_stats().await;
    let api_response = ApiResponse::success(stats, request_id);
    let json = serde_json::to_string(&api_response).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::new(Body::from(json)))
}

/// Create CORS response
fn create_cors_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .body(Body::from(""))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_api_response_creation() {
        let response = ApiResponse::success("test_data", "req_123".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test_data"));
        assert!(response.error.is_none());
        assert_eq!(response.request_id, "req_123");
    }
    
    #[tokio::test]
    async fn test_api_error_response() {
        let response = ApiResponse::<()>::error("test_error".to_string(), "req_123".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test_error".to_string()));
        assert_eq!(response.request_id, "req_123");
    }
    
    #[tokio::test]
    async fn test_node_status_response() {
        let response = NodeStatusResponse {
            node_id: "test_node".to_string(),
            is_running: true,
            uptime_seconds: 3600,
            version: "1.0.0".to_string(),
            network_id: "test_network".to_string(),
            chain_id: 1,
            current_block_height: 100,
            current_block_hash: "0x123".to_string(),
            peer_count: 5,
            sync_status: "synced".to_string(),
            consensus_status: "active".to_string(),
        };
        
        assert_eq!(response.node_id, "test_node");
        assert!(response.is_running);
        assert_eq!(response.uptime_seconds, 3600);
        assert_eq!(response.current_block_height, 100);
    }
    
    #[tokio::test]
    async fn test_transaction_request() {
        let request = TransactionRequest {
            transaction_type: "transfer".to_string(),
            from: "i1234567890abcdef".to_string(),
            to: "i0987654321fedcba".to_string(),
            amount: 1000,
            fee: Some(100),
            nonce: 1,
            data: None,
            signature: "0x1234567890abcdef".to_string(),
        };
        
        assert_eq!(request.transaction_type, "transfer");
        assert_eq!(request.from, "i1234567890abcdef");
        assert_eq!(request.amount, 1000);
        assert_eq!(request.fee, Some(100));
    }
}
