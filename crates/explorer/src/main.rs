//! IPPAN Block Explorer and API Gateway
//!
//! Provides a RESTful API for exploring the IPPAN blockchain.

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::get,
    Router,
};
use clap::Parser;
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[derive(Parser)]
#[command(name = "ippan-explorer")]
#[command(about = "IPPAN Block Explorer and API Gateway")]
#[command(version)]
struct Cli {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Bind port
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// IPPAN node RPC URL
    #[arg(long, default_value = "http://localhost:8080")]
    node_rpc: String,
}

#[derive(Clone)]
struct AppState {
    node_rpc: String,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let state = AppState {
        node_rpc: cli.node_rpc.clone(),
        client: reqwest::Client::new(),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/blocks", get(get_blocks))
        .route("/api/block/:id", get(get_block))
        .route("/api/transactions", get(get_transactions))
        .route("/api/transaction/:hash", get(get_transaction))
        .route("/api/validators", get(get_validators))
        .route("/api/validator/:id", get(get_validator))
        .route("/api/stats", get(get_stats))
        .route("/api/node/status", get(get_node_status))
        .route("/api/node/peers", get(get_node_peers))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state));

    let addr = format!("{}:{}", cli.host, cli.port);
    tracing::info!("🚀 IPPAN Explorer starting on {}", addr);
    tracing::info!("📡 Connected to node: {}", cli.node_rpc);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>IPPAN Block Explorer</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #4CAF50;
            padding-bottom: 10px;
        }
        .api-list {
            list-style: none;
            padding: 0;
        }
        .api-list li {
            margin: 10px 0;
            padding: 10px;
            background: #f9f9f9;
            border-left: 4px solid #4CAF50;
        }
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
        }
        .version {
            color: #666;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🔍 IPPAN Block Explorer API</h1>
        <p class="version">Version 1.0</p>
        
        <h2>Available Endpoints:</h2>
        <ul class="api-list">
            <li><code>GET /api/health</code> - Health check</li>
            <li><code>GET /api/blocks?page=1&limit=20</code> - List blocks</li>
            <li><code>GET /api/block/:id</code> - Get block by height or hash</li>
            <li><code>GET /api/transactions?page=1&limit=20</code> - List transactions</li>
            <li><code>GET /api/transaction/:hash</code> - Get transaction by hash</li>
            <li><code>GET /api/validators</code> - List validators</li>
            <li><code>GET /api/validator/:id</code> - Get validator info</li>
            <li><code>GET /api/stats</code> - Blockchain statistics</li>
            <li><code>GET /api/node/status</code> - Node status</li>
            <li><code>GET /api/node/peers</code> - Node peers</li>
        </ul>
        
        <h2>Example Usage:</h2>
        <pre><code>curl http://localhost:3000/api/blocks
curl http://localhost:3000/api/block/latest
curl http://localhost:3000/api/validators</code></pre>
    </div>
</body>
</html>
    "#,
    )
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "ippan-explorer"
    }))
}

#[derive(Deserialize)]
struct PaginationParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_page() -> usize {
    1
}
fn default_limit() -> usize {
    20
}

async fn get_blocks(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!(
            "{}/blocks?page={}&limit={}",
            state.node_rpc, params.page, params.limit
        ))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_block(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/block/{}", state.node_rpc, id))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_transactions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!(
            "{}/transactions?page={}&limit={}",
            state.node_rpc, params.page, params.limit
        ))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_transaction(
    State(state): State<Arc<AppState>>,
    Path(hash): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/transaction/{}", state.node_rpc, hash))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_validators(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/validators", state.node_rpc))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_validator(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/validator/{}", state.node_rpc, id))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/blockchain/info", state.node_rpc))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_node_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/node/status", state.node_rpc))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}

async fn get_node_peers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resp = state
        .client
        .get(format!("{}/node/peers", state.node_rpc))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to node: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse response: {e}"),
            )
        })?;

    Ok(Json(resp))
}
