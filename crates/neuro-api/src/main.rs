use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use neuro_core::*;
use neuro_ledger::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    models: Arc<RwLock<ModelRegistry>>,
    datasets: Arc<RwLock<DatasetRegistry>>,
    jobs: Arc<RwLock<JobMarket>>,
    proofs: Arc<RwLock<ProofStore>>,
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

#[tokio::main]
async fn main() {
    let state = AppState {
        models: Arc::new(RwLock::new(ModelRegistry::open("./data/models"))),
        datasets: Arc::new(RwLock::new(DatasetRegistry::open("./data/datasets"))),
        jobs: Arc::new(RwLock::new(JobMarket::open("./data/jobs"))),
        proofs: Arc::new(RwLock::new(ProofStore::open("./data/proofs"))),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/models", post(create_model))
        .route("/models/:id", get(get_model))
        .route("/datasets", post(create_dataset))
        .route("/datasets/:id", get(get_dataset))
        .route("/jobs", post(create_job))
        .route("/jobs/:id", get(get_job))
        .route("/bids", post(place_bid))
        .route("/proofs", post(submit_proof))
        .route("/proofs/:id", get(get_proof))
        .with_state(state);

    println!("Neuro API server starting on :3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health_check() -> StatusCode {
    StatusCode::OK
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
    state: axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
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
    
    let bid = job_market::Bid {
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
    state: axum::extract::State<AppState>,
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
