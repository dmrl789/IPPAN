// TODO: Implement Axum HTTP API endpoints
// - GET /health -> {status, peers, mempool_size}
// - GET /metrics (Prometheus exporter)
// - POST /tx -> accept binary/hex tx -> enqueue

use axum::{
    routing::{get, post},
    Router,
    Json,
    http::StatusCode,
    extract::State,
    body::Bytes,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use ippan_common::{Transaction, Result as IppanResult};
use crate::mempool::Mempool;
use crate::p2p::P2PNode;
use crate::metrics::Metrics;

#[derive(Clone)]
pub struct AppState {
    pub mempool: Arc<RwLock<Mempool>>,
    pub p2p: Arc<RwLock<P2PNode>>,
    pub metrics: Arc<Metrics>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/tx", post(tx_handler))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

async fn root_handler() -> &'static str {
    tracing::debug!("Root endpoint called");
    "IPPAN Node is running!"
}

async fn health_handler(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    tracing::debug!("Health endpoint called");
    
    let mempool = state.mempool.read().await;
    let p2p = state.p2p.read().await;
    
    let response = json!({
        "status": "healthy",
        "peers": p2p.peer_count(),
        "mempool_size": mempool.size(),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });
    
    tracing::debug!("Health response: {:?}", response);
    Json(response)
}

async fn metrics_handler(
    State(state): State<AppState>,
) -> (StatusCode, String) {
    let metrics = state.metrics.get_prometheus_metrics();
    (StatusCode::OK, metrics)
}

async fn tx_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> (StatusCode, String) {
    tracing::debug!("Received transaction request, body size: {}", body.len());
    
    // Try to deserialize as binary first
    let tx_result: IppanResult<Transaction> = match bincode::deserialize(&body) {
        Ok(tx) => {
            tracing::debug!("Successfully deserialized transaction");
            Ok(tx)
        },
        Err(e) => {
            tracing::debug!("Failed to deserialize as binary: {}", e);
            // Try hex encoding
            match hex::decode(&body) {
                Ok(hex_data) => {
                    tracing::debug!("Decoded hex data, size: {}", hex_data.len());
                    bincode::deserialize(&hex_data)
                        .map_err(|e| ippan_common::Error::Serialization(e.to_string()))
                },
                Err(_) => {
                    tracing::debug!("Failed to decode as hex");
                    Err(ippan_common::Error::Validation("Invalid transaction format".to_string()))
                },
            }
        }
    };

    match tx_result {
        Ok(tx) => {
            tracing::debug!("Transaction deserialized successfully, verifying...");
            
            // Verify transaction
            match tx.verify() {
                Ok(_) => {
                    tracing::debug!("Transaction verification successful");
                },
                Err(e) => {
                    tracing::debug!("Transaction verification failed: {}", e);
                    return (StatusCode::BAD_REQUEST, format!("Transaction verification failed: {}", e));
                }
            }

            // Add to mempool
            tracing::debug!("Adding transaction to mempool...");
            let mut mempool = state.mempool.write().await;
            match mempool.add_transaction(tx.clone()).await {
                Ok(true) => {
                    tracing::debug!("Transaction added to mempool successfully");
                    state.metrics.record_transaction_received();
                    
                    // Broadcast to P2P network
                    let mut p2p = state.p2p.write().await;
                    if let Err(e) = p2p.broadcast_transaction(&tx).await {
                        tracing::warn!("Failed to broadcast transaction: {}", e);
                    }
                    
                    (StatusCode::OK, "Transaction accepted".to_string())
                }
                Ok(false) => {
                    tracing::debug!("Transaction rejected by mempool");
                    (StatusCode::BAD_REQUEST, "Transaction rejected".to_string())
                },
                Err(e) => {
                    tracing::debug!("Mempool error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Mempool error: {}", e))
                },
            }
        }
        Err(e) => {
            tracing::debug!("Transaction deserialization failed: {}", e);
            (StatusCode::BAD_REQUEST, format!("Invalid transaction: {}", e))
        },
    }
}
