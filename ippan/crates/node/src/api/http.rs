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
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/tx", post(tx_handler))
        .with_state(state)
}

async fn health_handler(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let mempool = state.mempool.read().await;
    let p2p = state.p2p.read().await;
    
    Json(json!({
        "status": "healthy",
        "peers": p2p.peer_count(),
        "mempool_size": mempool.size(),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
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
    // Try to deserialize as binary first
    let tx_result: IppanResult<Transaction> = match bincode::deserialize(&body) {
        Ok(tx) => Ok(tx),
        Err(_) => {
            // Try hex encoding
            match hex::decode(&body) {
                Ok(hex_data) => bincode::deserialize(&hex_data)
                    .map_err(|e| ippan_common::Error::Serialization(e.to_string())),
                Err(_) => Err(ippan_common::Error::Validation("Invalid transaction format".to_string())),
            }
        }
    };

    match tx_result {
        Ok(tx) => {
            // Verify transaction
            if let Err(e) = tx.verify() {
                return (StatusCode::BAD_REQUEST, format!("Transaction verification failed: {}", e));
            }

            // Add to mempool
            let mut mempool = state.mempool.write().await;
            match mempool.add_transaction(tx.clone()).await {
                Ok(true) => {
                    state.metrics.record_transaction_received();
                    
                    // Broadcast to P2P network
                    let mut p2p = state.p2p.write().await;
                    if let Err(e) = p2p.broadcast_transaction(&tx).await {
                        tracing::warn!("Failed to broadcast transaction: {}", e);
                    }
                    
                    (StatusCode::OK, "Transaction accepted".to_string())
                }
                Ok(false) => (StatusCode::BAD_REQUEST, "Transaction rejected".to_string()),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Mempool error: {}", e)),
            }
        }
        Err(e) => (StatusCode::BAD_REQUEST, format!("Invalid transaction: {}", e)),
    }
}
