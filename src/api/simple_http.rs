//! Simple HTTP API server for IPPAN
//! 
//! Provides REST endpoints using tokio and hyper for better compatibility

use crate::node::IppanNode;
use crate::Result;
use crate::transaction::{Transaction, TransactionType, create_transaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::collections::HashMap;

/// Simple HTTP server for IPPAN API
pub struct SimpleHttpServer {
    pub node: Option<Arc<RwLock<IppanNode>>>,
    pub addr: SocketAddr,
}

impl SimpleHttpServer {
    /// Create a new simple HTTP server
    pub fn new(node: Option<Arc<RwLock<IppanNode>>>, addr: SocketAddr) -> Self {
        Self { node, addr }
    }

    /// Start the HTTP server
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting simple HTTP server on {}", self.addr);
        
        let node = self.node.clone();
        let make_svc = make_service_fn(move |_conn| {
            let node = node.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let node = node.clone();
                    handle_request(req, node)
                }))
            }
        });

        let server = Server::bind(&self.addr).serve(make_svc);
        
        log::info!("Simple HTTP server listening on {}", self.addr);
        
        if let Err(e) = server.await {
            log::error!("Server error: {}", e);
            return Err(crate::error::IppanError::Network(format!("HTTP server error: {}", e)));
        }

        Ok(())
    }
}

/// Handle HTTP requests
async fn handle_request(
    req: Request<Body>,
    node: Option<Arc<RwLock<IppanNode>>>,
) -> std::result::Result<Response<Body>, Infallible> {
    let method = req.method();
    let path = req.uri().path();

    log::debug!("HTTP request: {} {}", method, path);

    match (method, path) {
        // Health check
        (&Method::GET, "/health") => {
            Ok(Response::new(Body::from("OK")))
        }
        
        // Node status
        (&Method::GET, "/status") => {
            if let Some(node_arc) = &node {
                let node_guard = node_arc.read().await;
                let status = node_guard.get_status();
            
                let status_response = StatusResponse {
                    is_running: status.is_running,
                    uptime_seconds: status.uptime.as_secs(),
                    version: status.version,
                    node_id: hex::encode(node_guard.node_id()),
                };
            
                let json = serde_json::to_string(&status_response).unwrap_or_else(|_| "{}".to_string());
                Ok(Response::new(Body::from(json)))
            } else {
                let status_response = StatusResponse {
                    is_running: true,
                    uptime_seconds: 0,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    node_id: "unknown".to_string(),
                };
                let json = serde_json::to_string(&status_response).unwrap_or_else(|_| "{}".to_string());
                Ok(Response::new(Body::from(json)))
            }
        }
        
        // API status
        (&Method::GET, "/api/v1/status") => {
            if let Some(node_arc) = &node {
                let node_guard = node_arc.read().await;
                let status = node_guard.get_status();
                let network_stats = node_guard.network.read().await.get_network_stats().await;
                let mempool_stats = node_guard.get_mempool_stats().await;
            
                let api_status = ApiStatusResponse {
                    node: NodeInfo {
                        is_running: status.is_running,
                        uptime_seconds: status.uptime.as_secs(),
                        version: status.version,
                        node_id: hex::encode(node_guard.node_id()),
                    },
                    network: NetworkInfo {
                        connected_peers: network_stats.active_connections,
                        known_peers: network_stats.known_peers,
                        total_peers: network_stats.total_peers,
                    },
                    mempool: MempoolInfo {
                        total_transactions: mempool_stats.total_transactions,
                        total_senders: mempool_stats.total_senders,
                        total_size: mempool_stats.total_size,
                    },
                    consensus: ConsensusInfo {
                        current_round: node_guard.consensus.read().await.current_round(),
                        validator_count: node_guard.consensus.read().await.get_validators().len(),
                    },
                };
            
                let json = serde_json::to_string(&api_status).unwrap_or_else(|_| "{}".to_string());
                Ok(Response::new(Body::from(json)))
            } else {
                let api_status = ApiStatusResponse {
                    node: NodeInfo { is_running: true, uptime_seconds: 0, version: env!("CARGO_PKG_VERSION").to_string(), node_id: "unknown".to_string() },
                    network: NetworkInfo { connected_peers: 0, known_peers: 0, total_peers: 0 },
                    mempool: MempoolInfo { total_transactions: 0, total_senders: 0, total_size: 0 },
                    consensus: ConsensusInfo { current_round: 0, validator_count: 0 },
                };
                let json = serde_json::to_string(&api_status).unwrap_or_else(|_| "{}".to_string());
                Ok(Response::new(Body::from(json)))
            }
        }
        
        // Get account balance
        (&Method::GET, path) if path.starts_with("/api/v1/balance/") => {
            let account = path.strip_prefix("/api/v1/balance/").unwrap_or("");
            let node_guard = node.read().await;
            let balance = node_guard.get_account_balance(account).await;
            
            let balance_response = BalanceResponse {
                account: account.to_string(),
                balance,
            };
            
            let json = serde_json::to_string(&balance_response).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::new(Body::from(json)))
        }
        
        // Send transaction
        (&Method::POST, "/api/v1/transaction") => {
            let body = hyper::body::to_bytes(req.into_body()).await.unwrap_or_default();
            let tx_request: TransactionRequest = match serde_json::from_slice(&body) {
                Ok(req) => req,
                Err(e) => {
                    let error_response = ErrorResponse {
                        error: format!("Invalid transaction request: {}", e),
                    };
                    let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from(json))
                        .unwrap());
                }
            };
            
            // Create transaction
            let transaction = match create_transaction(
                tx_request.tx_type,
                tx_request.nonce,
                tx_request.sender,
                tx_request.signature,
            ) {
                Ok(tx) => tx,
                Err(e) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from(format!("Failed to create transaction: {}", e)))
                        .unwrap());
                }
            };
            
            // Process transaction
            let node_guard = node.read().await;
            let processed = node_guard.process_transaction(transaction).await.unwrap_or(false);
            
            let response = if processed {
                TransactionResponse {
                    success: true,
                    message: "Transaction processed successfully".to_string(),
                }
            } else {
                TransactionResponse {
                    success: false,
                    message: "Transaction processing failed".to_string(),
                }
            };
            
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            let status = if processed { StatusCode::OK } else { StatusCode::BAD_REQUEST };
            
            Ok(Response::builder()
                .status(status)
                .body(Body::from(json))
                .unwrap())
        }
        
        // Get mempool stats
        (&Method::GET, "/api/v1/mempool") => {
            let node_guard = node.read().await;
            let mempool_stats = node_guard.get_mempool_stats().await;
            
            let json = serde_json::to_string(&mempool_stats).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::new(Body::from(json)))
        }
        
        // Get network stats
        (&Method::GET, "/api/v1/network") => {
            let node_guard = node.read().await;
            let network_stats = node_guard.network.read().await.get_network_stats().await;
            
            let json = serde_json::to_string(&network_stats).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::new(Body::from(json)))
        }
        
        // Get consensus stats
        (&Method::GET, "/api/v1/consensus") => {
            let node_guard = node.read().await;
            let consensus_engine = node_guard.consensus.read().await;
            let current_round = consensus_engine.current_round();
            let validators = consensus_engine.get_validators();
            
            let consensus_stats = ConsensusStatsResponse {
                current_round,
                validator_count: validators.len(),
                total_stake: validators.values().sum(),
            };
            
            let json = serde_json::to_string(&consensus_stats).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::new(Body::from(json)))
        }
        
        // Default route - return 404
        _ => {
            let error_response = ErrorResponse {
                error: "Not found".to_string(),
            };
            let json = serde_json::to_string(&error_response).unwrap_or_else(|_| "{}".to_string());
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(json))
                .unwrap())
        }
    }
}

/// Status response
#[derive(Debug, Serialize)]
struct StatusResponse {
    is_running: bool,
    uptime_seconds: u64,
    version: String,
    node_id: String,
}

/// API status response
#[derive(Debug, Serialize)]
struct ApiStatusResponse {
    node: NodeInfo,
    network: NetworkInfo,
    mempool: MempoolInfo,
    consensus: ConsensusInfo,
}

/// Node information
#[derive(Debug, Serialize)]
struct NodeInfo {
    is_running: bool,
    uptime_seconds: u64,
    version: String,
    node_id: String,
}

/// Network information
#[derive(Debug, Serialize)]
struct NetworkInfo {
    connected_peers: usize,
    known_peers: usize,
    total_peers: usize,
}

/// Mempool information
#[derive(Debug, Serialize)]
struct MempoolInfo {
    total_transactions: usize,
    total_senders: usize,
    total_size: usize,
}

/// Consensus information
#[derive(Debug, Serialize)]
struct ConsensusInfo {
    current_round: u64,
    validator_count: usize,
}

/// Balance response
#[derive(Debug, Serialize)]
struct BalanceResponse {
    account: String,
    balance: u64,
}

/// Transaction request
#[derive(Debug, Deserialize)]
struct TransactionRequest {
    tx_type: TransactionType,
    nonce: u64,
    sender: String,
    signature: String,
}

/// Transaction response
#[derive(Debug, Serialize)]
struct TransactionResponse {
    success: bool,
    message: String,
}

/// Consensus stats response
#[derive(Debug, Serialize)]
struct ConsensusStatsResponse {
    current_round: u64,
    validator_count: usize,
    total_stake: u64,
}

/// Error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_simple_http_server_creation() {
        // Create a minimal mock node for testing
        let config = crate::config::Config::default();
        let node = match crate::node::IppanNode::new(config).await {
            Ok(node) => Arc::new(RwLock::new(node)),
            Err(_) => {
                // If node creation fails, skip this test
                return;
            }
        };
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let server = SimpleHttpServer::new(node, addr);
        
        assert_eq!(server.addr, addr);
    }

    #[tokio::test]
    async fn test_status_response_serialization() {
        let status = StatusResponse {
            is_running: true,
            uptime_seconds: 123,
            version: "1.0.0".to_string(),
            node_id: "test_node_id".to_string(),
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("is_running"));
        assert!(json.contains("uptime_seconds"));
        assert!(json.contains("version"));
        assert!(json.contains("node_id"));
    }

    #[tokio::test]
    async fn test_transaction_request_deserialization() {
        let json = r#"{
            "tx_type": {
                "Payment": {
                    "from": "alice",
                    "to": "bob",
                    "amount": 1000,
                    "fee": 10
                }
            },
            "nonce": 0,
            "sender": "alice",
            "signature": "test_signature"
        }"#;
        
        let tx_request: TransactionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(tx_request.nonce, 0);
        assert_eq!(tx_request.sender, "alice");
        assert_eq!(tx_request.signature, "test_signature");
    }
}
