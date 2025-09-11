use axum::{
    routing::{get, post, delete},
    http::StatusCode,
    Json, Router,
};
use neuro_core::*;
use neuro_ledger::{model_registry::ModelRegistry, dataset_registry::DatasetRegistry, job_market::JobMarket, proofs::ProofStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use sha2::Digest;

#[derive(Clone)]
struct AppState {
    models: Arc<RwLock<ModelRegistry>>,
    datasets: Arc<RwLock<DatasetRegistry>>,
    jobs: Arc<RwLock<JobMarket>>,
    proofs: Arc<RwLock<ProofStore>>,
    files: Arc<RwLock<HashMap<String, FileData>>>,
    wallets: Arc<RwLock<HashMap<String, WalletData>>>,
}

#[derive(Clone)]
struct FileData {
    name: String,
    size: u64,
    hash: String,
    content: Vec<u8>,
}

#[derive(Clone)]
struct WalletData {
    address: String,
    balance: u128,
    staked: u128,
    rewards: u128,
    pending_transactions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CreateModelRequest {
    pub owner: String, // hex address
    pub arch_id: u32,
    pub version: u32,
    pub weights_hash: String, // hex hash
    pub size_bytes: u64,
    pub license_id: u32,
}

#[derive(Serialize, Deserialize)]
struct CreateJobRequest {
    pub model_ref: String, // hex hash
    pub input_commit: String, // hex hash
    pub max_latency_ms: u32,
    pub region: String,
    pub max_price_ipn: u128,
    pub escrow_ipn: u128,
    pub privacy: String, // "open", "tee", "zk"
    pub bid_window_ms: u16,
}

#[derive(Serialize, Deserialize)]
struct PlaceBidRequest {
    pub job_id: String, // hex hash
    pub executor_id: String, // hex address
    pub price_ipn: u128,
    pub est_latency_ms: u32,
    pub tee: bool,
}

#[derive(Serialize, Deserialize)]
struct SendTransactionRequest {
    pub to: String,
    pub amount: u128,
    pub fee: Option<u128>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        models: Arc::new(RwLock::new(ModelRegistry::open("./data/models"))),
        datasets: Arc::new(RwLock::new(DatasetRegistry::open("./data/datasets"))),
        jobs: Arc::new(RwLock::new(JobMarket::open("./data/jobs"))),
        proofs: Arc::new(RwLock::new(ProofStore::open("./data/proofs"))),
        files: Arc::new(RwLock::new(HashMap::new())),
        wallets: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/status", get(blockchain_status))
        .route("/models", post(create_model))
        .route("/models/:id", get(get_model))
        .route("/datasets", post(create_dataset))
        .route("/datasets/:id", get(get_dataset))
        .route("/jobs", post(create_job))
        .route("/jobs/:id", get(get_job))
        .route("/bids", post(place_bid))
        .route("/proofs", post(submit_proof))
        .route("/proofs/:id", get(get_proof))
        .route("/storage/files", post(store_file))
        .route("/storage/files", get(list_files))
        .route("/storage/files/:file_id", get(get_file))
        .route("/storage/files/:file_id", delete(delete_file))
        .route("/wallet/balance", get(get_wallet_balance))
        .route("/wallet/send", post(send_transaction))
        .route("/consensus/round", get(get_consensus_round))
        .route("/consensus/validators", get(get_validators))
        .with_state(state);

    println!("Neuro API server starting on :3001");
    axum::Server::bind(&"0.0.0.0:3001".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().timestamp(),
        "version": "1.0.0",
        "success": true
    }))
}

async fn create_model(
    state: axum::extract::State<AppState>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut models = state.models.write().await;
    
    let owner = hex::decode(&req.owner).map_err(|_| StatusCode::BAD_REQUEST)?;
    let weights_hash = hex::decode(&req.weights_hash).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let asset = ModelAsset {
        id: blake3_hash(&weights_hash),
        owner: owner.try_into().map_err(|_| StatusCode::BAD_REQUEST)?,
        arch_id: req.arch_id,
        version: req.version,
        weights_hash: weights_hash.try_into().map_err(|_| StatusCode::BAD_REQUEST)?,
        size_bytes: req.size_bytes,
        train_parent: None,
        train_config: [0; 32],
        license_id: req.license_id,
        metrics: vec![],
        provenance: vec![],
        created_at: HashTimer::default(),
    };

    models.put(&asset).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({
        "id": hex::encode(asset.id),
        "success": true
    })))
}

async fn get_model(
    state: axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let models = state.models.read().await;
    let hash = hex::decode(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let hash: Hash = hash.try_into().map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match models.get(&hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        Some(asset) => Ok(Json(serde_json::json!({
            "id": hex::encode(asset.id),
            "owner": hex::encode(asset.owner),
            "arch_id": asset.arch_id,
            "version": asset.version,
            "weights_hash": hex::encode(asset.weights_hash),
            "size_bytes": asset.size_bytes,
            "license_id": asset.license_id,
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_dataset(
    _state: axum::extract::State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: implement dataset creation
    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_dataset(
    _state: axum::extract::State<AppState>,
    _id: axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: implement dataset retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn create_job(
    state: axum::extract::State<AppState>,
    Json(req): Json<CreateJobRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut jobs = state.jobs.write().await;
    
    let model_ref = hex::decode(&req.model_ref).map_err(|_| StatusCode::BAD_REQUEST)?;
    let input_commit = hex::decode(&req.input_commit).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let privacy = match req.privacy.as_str() {
        "open" => PrivacyMode::Open,
        "tee" => PrivacyMode::TEE,
        "zk" => PrivacyMode::Zk,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    let job = InferenceJob {
        id: blake3_hash(&input_commit),
        model_ref: model_ref.try_into().map_err(|_| StatusCode::BAD_REQUEST)?,
        input_commit: input_commit.try_into().map_err(|_| StatusCode::BAD_REQUEST)?,
        sla: Sla {
            max_latency_ms: req.max_latency_ms,
            region: req.region,
            price_cap_ipn: req.max_price_ipn,
        },
        privacy,
        bid_window_ms: req.bid_window_ms,
        max_price_ipn: req.max_price_ipn,
        escrow_ipn: req.escrow_ipn,
        created_at: HashTimer::default(),
    };

    jobs.post_job(&job).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({
        "id": hex::encode(job.id),
        "success": true
    })))
}

async fn get_job(
    _state: axum::extract::State<AppState>,
    _id: axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: implement job retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn place_bid(
    state: axum::extract::State<AppState>,
    Json(req): Json<PlaceBidRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut jobs = state.jobs.write().await;
    
    let job_id = hex::decode(&req.job_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let executor_id = hex::decode(&req.executor_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let bid = neuro_ledger::job_market::Bid {
        job_id: job_id.try_into().map_err(|_| StatusCode::BAD_REQUEST)?,
        executor_id: executor_id.try_into().map_err(|_| StatusCode::BAD_REQUEST)?,
        price_ipn: req.price_ipn,
        est_latency_ms: req.est_latency_ms,
        tee: req.tee,
    };

    jobs.place_bid(&bid).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({"success": true})))
}

async fn submit_proof(
    _state: axum::extract::State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: implement proof submission
    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_proof(
    _state: axum::extract::State<AppState>,
    _id: axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: implement proof retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}

// Blockchain status endpoint
async fn blockchain_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "node_id": "test_node_001",
        "status": "running",
        "current_block": 12345,
        "total_transactions": 1000000,
        "network_peers": 50,
        "uptime_seconds": 86400,
        "version": "1.0.0"
    }))
}

// File storage endpoints
async fn store_file(
    state: axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let mut files = state.files.write().await;
    
    // For demo purposes, create a dummy file
    let file_id = Uuid::new_v4().to_string();
    let dummy_data = b"dummy file content for testing";
    let hash = format!("{:x}", sha2::Sha256::digest(dummy_data));
    
    let file_data = FileData {
        name: "test_file.txt".to_string(),
        size: dummy_data.len() as u64,
        hash: hash.clone(),
        content: dummy_data.to_vec(),
    };
    
    files.insert(file_id.clone(), file_data);
    
    Json(serde_json::json!({
        "file_id": file_id,
        "name": "test_file.txt",
        "hash": hash,
        "size": dummy_data.len(),
        "success": true
    }))
}

async fn list_files(
    state: axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let files = state.files.read().await;
    let file_list: Vec<_> = files.iter().map(|(id, file)| {
        serde_json::json!({
            "file_id": id,
            "name": file.name,
            "size": file.size,
            "hash": file.hash
        })
    }).collect();
    
    Json(serde_json::json!({
        "files": file_list,
        "success": true
    }))
}

async fn get_file(
    state: axum::extract::State<AppState>,
    axum::extract::Path(file_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let files = state.files.read().await;
    
    if let Some(file_data) = files.get(&file_id) {
        Ok(Json(serde_json::json!({
            "file_id": file_id,
            "name": file_data.name,
            "size": file_data.size,
            "hash": file_data.hash,
            "success": true
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn delete_file(
    state: axum::extract::State<AppState>,
    axum::extract::Path(file_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut files = state.files.write().await;
    
    if files.remove(&file_id).is_some() {
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "File deleted successfully"
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Wallet endpoints
async fn get_wallet_balance(
    _state: axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let _wallets = _state.wallets.read().await;
    
    // For demo purposes, return a default wallet
    Json(serde_json::json!({
        "address": "0x1234567890abcdef1234567890abcdef12345678",
        "balance": 1000000000000000000u128, // 1 IPN
        "staked": 500000000000000000u128,   // 0.5 IPN
        "rewards": 100000000000000000u128,  // 0.1 IPN
        "pending_transactions": [],
        "success": true
    }))
}

async fn send_transaction(
    _state: axum::extract::State<AppState>,
    Json(req): Json<SendTransactionRequest>,
) -> Json<serde_json::Value> {
    // For demo purposes, simulate transaction processing
    let tx_hash = format!("0x{:x}", sha2::Sha256::digest(format!("{}{}{}", req.to, req.amount, req.fee.unwrap_or(0)).as_bytes()));
    
    Json(serde_json::json!({
        "transaction_hash": tx_hash,
        "status": "pending",
        "to": req.to,
        "amount": req.amount,
        "fee": req.fee,
        "success": true
    }))
}

// Consensus endpoints
async fn get_consensus_round() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "round_number": 12345,
        "validators": [
            {
                "address": "0xvalidator1",
                "stake": 1000000000000000000u128,
                "status": "active"
            },
            {
                "address": "0xvalidator2", 
                "stake": 800000000000000000u128,
                "status": "active"
            }
        ],
        "round_status": "finalized",
        "success": true
    }))
}

async fn get_validators() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "validators": [
            {
                "address": "0xvalidator1",
                "stake": 1000000000000000000u128,
                "status": "active",
                "uptime": 99.5
            },
            {
                "address": "0xvalidator2",
                "stake": 800000000000000000u128, 
                "status": "active",
                "uptime": 98.2
            },
            {
                "address": "0xvalidator3",
                "stake": 600000000000000000u128,
                "status": "active", 
                "uptime": 97.8
            }
        ],
        "total_stake": 2400000000000000000u128,
        "success": true
    }))
}
