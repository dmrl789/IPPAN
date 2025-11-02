//! API implementation for AI Registry

use crate::{
    fees::FeeManager, governance::GovernanceManager, registry::ModelRegistry, types::*,
    FeeCalculation, FeeStats,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use ippan_ai_core::types::{ModelId, ModelMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

type ApiResult<T> = std::result::Result<T, StatusCode>;

/// API state
#[derive(Clone)]
pub struct ApiState {
    /// Model registry
    pub registry: Arc<RwLock<ModelRegistry>>,
    /// Governance manager
    pub governance: Arc<RwLock<GovernanceManager>>,
    /// Fee manager
    pub fees: Arc<RwLock<FeeManager>>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    /// Success flag
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message
    pub error: Option<String>,
}

/// Model registration request
#[derive(Debug, Deserialize)]
pub struct ModelRegistrationRequest {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model hash
    pub hash: String,
    /// Model architecture
    pub architecture: String,
    /// Input shape
    pub input_shape: Vec<usize>,
    /// Output shape
    pub output_shape: Vec<usize>,
    /// Parameter count
    pub parameter_count: u64,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Model description
    pub description: Option<String>,
    /// Model license
    pub license: Option<String>,
    /// Source URL
    pub source_url: Option<String>,
    /// Model category
    pub category: ModelCategory,
    /// Model tags
    pub tags: Vec<String>,
    /// Registrant address
    pub registrant: String,
}

/// Governance proposal request
#[derive(Debug, Deserialize)]
pub struct GovernanceProposalRequest {
    /// Proposal type
    pub proposal_type: ProposalType,
    /// Proposal title
    pub title: String,
    /// Proposal description
    pub description: String,
    /// Proposer address
    pub proposer: String,
    /// Proposal data
    pub data: ProposalData,
}

/// Vote request
#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    /// Voter address
    pub voter: String,
    /// Vote choice
    pub choice: VoteChoice,
    /// Vote justification
    pub justification: Option<String>,
}

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query
    pub q: String,
    /// Category filter
    pub category: Option<ModelCategory>,
    /// Status filter
    pub status: Option<RegistrationStatus>,
    /// Limit
    pub limit: Option<usize>,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/models", post(register_model))
        .route("/models/:name", get(get_model))
        .route("/models/search", get(search_models))
        .route("/models/:name/status", post(update_model_status))
        .route("/models/:name/stats", get(get_model_stats))
        .route("/proposals", post(create_proposal))
        .route("/proposals/:id", get(get_proposal))
        .route("/proposals/:id/vote", post(vote_on_proposal))
        .route("/proposals/:id/execute", post(execute_proposal))
        .route("/proposals", get(list_proposals))
        .route("/fees/calculate", post(calculate_fee))
        .route("/fees/stats", get(get_fee_stats))
        .route("/stats", get(get_registry_stats))
        .with_state(state)
}

/// Register a new model
async fn register_model(
    State(state): State<ApiState>,
    Json(request): Json<ModelRegistrationRequest>,
) -> ApiResult<Json<ApiResponse<ModelRegistration>>> {
    let ModelRegistrationRequest {
        name,
        version,
        hash,
        architecture,
        input_shape,
        output_shape,
        parameter_count,
        size_bytes,
        description,
        license,
        source_url,
        category,
        tags,
        registrant,
    } = request;

    info!("API: Registering model: {}", name);

    let model_id = ModelId {
        name: name.clone(),
        version: version.clone(),
        hash: hash.clone(),
    };

    let timestamp = chrono::Utc::now().timestamp() as u64;
    let metadata = ModelMetadata {
        id: model_id.clone(),
        name: name.clone(),
        version: version.clone(),
        description: description
            .clone()
            .unwrap_or_else(|| "No description provided".to_string()),
        author: registrant.clone(),
        license: license.clone().unwrap_or_else(|| "unspecified".to_string()),
        tags: tags.clone(),
        created_at: timestamp,
        updated_at: timestamp,
        architecture,
        input_shape,
        output_shape,
        size_bytes,
        parameter_count,
    };

    let mut registry = state.registry.write().await;
    match registry
        .register_model(
            model_id,
            metadata,
            registrant,
            category,
            description,
            license,
            source_url,
            tags,
        )
        .await
    {
        Ok(registration) => {
            info!("API: Model registered successfully");
            Ok(Json(ApiResponse {
                success: true,
                data: Some(registration),
                error: None,
            }))
        }
        Err(e) => {
            error!("API: Model registration failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Get model by name
async fn get_model(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> ApiResult<Json<ApiResponse<ModelRegistration>>> {
    info!("API: Getting model: {}", name);

    let model_id = ModelId {
        name,
        version: String::new(), // We'll need to handle versioning properly
        hash: String::new(),
    };

    let mut registry = state.registry.write().await;
    match registry.get_model_registration(&model_id).await {
        Ok(Some(registration)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(registration),
            error: None,
        })),
        Ok(None) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Model not found".to_string()),
        })),
        Err(e) => {
            error!("API: Error getting model: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Search models
async fn search_models(
    State(state): State<ApiState>,
    Query(params): Query<SearchQuery>,
) -> ApiResult<Json<ApiResponse<Vec<ModelRegistration>>>> {
    info!("API: Searching models: {}", params.q);

    let registry = state.registry.read().await;
    match registry
        .search_models(&params.q, params.category, params.status, params.limit)
        .await
    {
        Ok(models) => Ok(Json(ApiResponse {
            success: true,
            data: Some(models),
            error: None,
        })),
        Err(e) => {
            error!("API: Error searching models: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Update model status
async fn update_model_status(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(status): Json<RegistrationStatus>,
) -> ApiResult<Json<ApiResponse<()>>> {
    info!("API: Updating model status: {} -> {:?}", name, status);

    let model_id = ModelId {
        name,
        version: String::new(),
        hash: String::new(),
    };

    let mut registry = state.registry.write().await;
    match registry.update_model_status(&model_id, status).await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        })),
        Err(e) => {
            error!("API: Error updating model status: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Get model statistics
async fn get_model_stats(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> ApiResult<Json<ApiResponse<ModelUsageStats>>> {
    info!("API: Getting model stats: {}", name);

    let model_id = ModelId {
        name,
        version: String::new(),
        hash: String::new(),
    };

    let registry = state.registry.read().await;
    match registry.get_model_usage_stats(&model_id).await {
        Ok(Some(stats)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
        })),
        Ok(None) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Model statistics not found".to_string()),
        })),
        Err(e) => {
            error!("API: Error getting model stats: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Create governance proposal
async fn create_proposal(
    State(state): State<ApiState>,
    Json(request): Json<GovernanceProposalRequest>,
) -> ApiResult<Json<ApiResponse<GovernanceProposal>>> {
    info!("API: Creating proposal: {}", request.title);

    let mut governance = state.governance.write().await;
    match governance
        .create_proposal(
            request.proposal_type,
            request.title,
            request.description,
            request.proposer,
            request.data,
        )
        .await
    {
        Ok(proposal) => {
            info!("API: Proposal created successfully");
            Ok(Json(ApiResponse {
                success: true,
                data: Some(proposal),
                error: None,
            }))
        }
        Err(e) => {
            error!("API: Proposal creation failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Get proposal by ID
async fn get_proposal(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<GovernanceProposal>>> {
    info!("API: Getting proposal: {}", id);

    let mut governance = state.governance.write().await;
    match governance.get_proposal(&id).await {
        Ok(Some(proposal)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(proposal),
            error: None,
        })),
        Ok(None) => Ok(Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Proposal not found".to_string()),
        })),
        Err(e) => {
            error!("API: Error getting proposal: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Vote on proposal
async fn vote_on_proposal(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(request): Json<VoteRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    info!("API: Voting on proposal: {}", id);

    let mut governance = state.governance.write().await;
    match governance
        .vote_on_proposal(&id, request.voter, request.choice, request.justification)
        .await
    {
        Ok(()) => {
            info!("API: Vote recorded successfully");
            Ok(Json(ApiResponse {
                success: true,
                data: Some(()),
                error: None,
            }))
        }
        Err(e) => {
            error!("API: Vote failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Execute proposal
async fn execute_proposal(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    info!("API: Executing proposal: {}", id);

    let mut governance = state.governance.write().await;
    match governance.execute_proposal(&id).await {
        Ok(()) => {
            info!("API: Proposal executed successfully");
            Ok(Json(ApiResponse {
                success: true,
                data: Some(()),
                error: None,
            }))
        }
        Err(e) => {
            error!("API: Proposal execution failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// List proposals
async fn list_proposals(
    State(state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<Vec<GovernanceProposal>>>> {
    info!("API: Listing proposals");

    let governance = state.governance.read().await;
    match governance.list_active_proposals().await {
        Ok(proposals) => Ok(Json(ApiResponse {
            success: true,
            data: Some(proposals),
            error: None,
        })),
        Err(e) => {
            error!("API: Error listing proposals: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Calculate fee
async fn calculate_fee(
    State(state): State<ApiState>,
    Json(request): Json<FeeCalculationRequest>,
) -> ApiResult<Json<ApiResponse<FeeCalculation>>> {
    info!("API: Calculating fee: {:?}", request.fee_type);

    let fees = state.fees.read().await;
    match fees.calculate_fee(
        request.fee_type,
        request.model_metadata.as_ref(),
        request.units,
        request.additional_data,
    ) {
        Ok(calculation) => Ok(Json(ApiResponse {
            success: true,
            data: Some(calculation),
            error: None,
        })),
        Err(e) => {
            error!("API: Fee calculation failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Get fee statistics
async fn get_fee_stats(State(state): State<ApiState>) -> ApiResult<Json<ApiResponse<FeeStats>>> {
    info!("API: Getting fee statistics");

    let fees = state.fees.read().await;
    let stats = fees.get_fee_stats().clone();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(stats),
        error: None,
    }))
}

/// Get registry statistics
async fn get_registry_stats(
    State(state): State<ApiState>,
) -> ApiResult<Json<ApiResponse<RegistryStats>>> {
    info!("API: Getting registry statistics");

    let mut registry = state.registry.write().await;
    match registry.get_registry_stats().await {
        Ok(stats) => Ok(Json(ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
        })),
        Err(e) => {
            error!("API: Error getting registry stats: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Fee calculation request
#[derive(Debug, Deserialize)]
pub struct FeeCalculationRequest {
    /// Fee type
    pub fee_type: FeeType,
    /// Model metadata (optional)
    pub model_metadata: Option<ModelMetadata>,
    /// Units (optional)
    pub units: Option<u64>,
    /// Additional data (optional)
    pub additional_data: Option<HashMap<String, String>>,
}
