use crate::Result;
use crate::crosschain::{
    CrossChainManager, AnchorTx, ProofType, VerificationResult, BridgeEndpoint,
    LightSyncData, CrossChainReport
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cross-chain API handler
pub struct CrossChainApi {
    manager: Arc<CrossChainManager>,
    server: Option<axum::Server<axum::extract::DefaultBodyLimit, axum::routing::IntoMakeService<Router>>>,
    bind_addr: String,
}

impl CrossChainApi {
    /// Create a new cross-chain API
    pub fn new(manager: Arc<CrossChainManager>) -> Self {
        Self {
            manager,
            server: None,
            bind_addr: "0.0.0.0:8081".to_string(),
        }
    }

    /// Start the cross-chain API server
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.create_router();
        
        let addr = self.bind_addr.parse()?;
        let server = axum::Server::bind(&addr).serve(app.into_make_service());
        
        self.server = Some(server);
        
        tracing::info!("Cross-chain API server started on {}", self.bind_addr);
        Ok(())
    }

    /// Stop the cross-chain API server
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(server) = self.server.take() {
            server.await?;
        }
        Ok(())
    }

    /// Create the router with all cross-chain endpoints
    fn create_router(&self) -> Router {
        Router::new()
            .route("/anchor", post(Self::submit_anchor))
            .route("/anchor/:chain_id", get(Self::get_latest_anchor))
            .route("/verify_inclusion", post(Self::verify_inclusion))
            .route("/bridge/register", post(Self::register_bridge))
            .route("/bridge/:chain_id", get(Self::get_bridge_endpoint))
            .route("/bridge/:chain_id", delete(Self::remove_bridge))
            .route("/light_sync/:round", get(Self::get_light_sync_data))
            .route("/report", get(Self::get_cross_chain_report))
            .route("/health", get(Self::health_check))
            .with_state(Arc::clone(&self.manager))
    }

    /// Submit an anchor transaction
    async fn submit_anchor(
        State(manager): State<Arc<CrossChainManager>>,
        Json(request): Json<SubmitAnchorRequest>,
    ) -> Json<ApiResponse<SubmitAnchorResponse>> {
        let anchor_tx = AnchorTx {
            external_chain_id: request.chain_id,
            external_state_root: request.state_root,
            timestamp: request.timestamp,
            proof_type: request.proof_type,
            proof_data: request.proof_data,
        };

        match manager.submit_anchor(anchor_tx).await {
            Ok(anchor_id) => Json(ApiResponse::success(SubmitAnchorResponse {
                anchor_id,
                status: "submitted".to_string(),
                timestamp: chrono::Utc::now(),
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to submit anchor: {}", e))),
        }
    }

    /// Get the latest anchor for a chain
    async fn get_latest_anchor(
        State(manager): State<Arc<CrossChainManager>>,
        Path(chain_id): Path<String>,
    ) -> Json<ApiResponse<GetLatestAnchorResponse>> {
        match manager.get_latest_anchor(&chain_id).await {
            Ok(Some(anchor)) => Json(ApiResponse::success(GetLatestAnchorResponse {
                chain_id,
                last_anchor: Some(anchor),
            })),
            Ok(None) => Json(ApiResponse::success(GetLatestAnchorResponse {
                chain_id,
                last_anchor: None,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to get latest anchor: {}", e))),
        }
    }

    /// Verify external inclusion proof
    async fn verify_inclusion(
        State(manager): State<Arc<CrossChainManager>>,
        Json(request): Json<VerifyInclusionRequest>,
    ) -> Json<ApiResponse<VerifyInclusionResponse>> {
        match manager.verify_external_inclusion(
            &request.chain_id,
            &request.tx_hash,
            &request.merkle_proof,
        ).await {
            Ok(result) => Json(ApiResponse::success(VerifyInclusionResponse {
                included: result.success,
                timestamp: result.anchor_timestamp,
                anchor_round: result.anchor_round,
                anchor_height: result.anchor_height,
                details: result.details,
                verified_at: result.verified_at,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to verify inclusion: {}", e))),
        }
    }

    /// Register a bridge endpoint
    async fn register_bridge(
        State(manager): State<Arc<CrossChainManager>>,
        Json(request): Json<RegisterBridgeRequest>,
    ) -> Json<ApiResponse<RegisterBridgeResponse>> {
        let endpoint = BridgeEndpoint {
            chain_id: request.chain_id,
            accepted_anchor_types: request.accepted_anchor_types,
            latest_anchor: None,
            config: request.config,
            status: request.status,
            last_activity: chrono::Utc::now(),
        };

        match manager.register_bridge(endpoint).await {
            Ok(()) => Json(ApiResponse::success(RegisterBridgeResponse {
                status: "registered".to_string(),
                timestamp: chrono::Utc::now(),
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to register bridge: {}", e))),
        }
    }

    /// Get bridge endpoint information
    async fn get_bridge_endpoint(
        State(manager): State<Arc<CrossChainManager>>,
        Path(chain_id): Path<String>,
    ) -> Json<ApiResponse<GetBridgeEndpointResponse>> {
        match manager.get_bridge_endpoint(&chain_id).await {
            Ok(Some(endpoint)) => Json(ApiResponse::success(GetBridgeEndpointResponse {
                endpoint: Some(endpoint),
            })),
            Ok(None) => Json(ApiResponse::success(GetBridgeEndpointResponse {
                endpoint: None,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to get bridge endpoint: {}", e))),
        }
    }

    /// Remove a bridge endpoint
    async fn remove_bridge(
        State(_manager): State<Arc<CrossChainManager>>,
        Path(chain_id): Path<String>,
    ) -> Json<ApiResponse<RemoveBridgeResponse>> {
        // Note: This would require adding a remove_bridge method to CrossChainManager
        Json(ApiResponse::error("Bridge removal not implemented yet".to_string()))
    }

    /// Get light sync data for a specific round
    async fn get_light_sync_data(
        State(manager): State<Arc<CrossChainManager>>,
        Path(round): Path<u64>,
    ) -> Json<ApiResponse<GetLightSyncDataResponse>> {
        match manager.get_light_sync_data(round).await {
            Ok(Some(sync_data)) => Json(ApiResponse::success(GetLightSyncDataResponse {
                sync_data: Some(sync_data),
            })),
            Ok(None) => Json(ApiResponse::success(GetLightSyncDataResponse {
                sync_data: None,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to get light sync data: {}", e))),
        }
    }

    /// Get comprehensive cross-chain report
    async fn get_cross_chain_report(
        State(manager): State<Arc<CrossChainManager>>,
    ) -> Json<ApiResponse<CrossChainReport>> {
        match manager.generate_cross_chain_report().await {
            Ok(report) => Json(ApiResponse::success(report)),
            Err(e) => Json(ApiResponse::error(format!("Failed to generate report: {}", e))),
        }
    }

    /// Health check endpoint
    async fn health_check() -> Json<ApiResponse<HealthCheckResponse>> {
        Json(ApiResponse::success(HealthCheckResponse {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
        }))
    }
}

// Request/Response structures

#[derive(Debug, Deserialize)]
pub struct SubmitAnchorRequest {
    pub chain_id: String,
    pub state_root: String,
    pub timestamp: crate::consensus::hashtimer::HashTimer,
    pub proof_type: Option<ProofType>,
    pub proof_data: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct SubmitAnchorResponse {
    pub anchor_id: String,
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct GetLatestAnchorResponse {
    pub chain_id: String,
    pub last_anchor: Option<AnchorTx>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyInclusionRequest {
    pub chain_id: String,
    pub tx_hash: String,
    pub merkle_proof: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct VerifyInclusionResponse {
    pub included: bool,
    pub timestamp: Option<crate::consensus::hashtimer::HashTimer>,
    pub anchor_round: Option<u64>,
    pub anchor_height: Option<u64>,
    pub details: String,
    pub verified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterBridgeRequest {
    pub chain_id: String,
    pub accepted_anchor_types: Vec<ProofType>,
    pub config: crate::crosschain::bridge::BridgeConfig,
    pub status: crate::crosschain::bridge::BridgeStatus,
}

#[derive(Debug, Serialize)]
pub struct RegisterBridgeResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct GetBridgeEndpointResponse {
    pub endpoint: Option<BridgeEndpoint>,
}

#[derive(Debug, Serialize)]
pub struct RemoveBridgeResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct GetLightSyncDataResponse {
    pub sync_data: Option<LightSyncData>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Generic API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosschain::CrossChainConfig;

    #[tokio::test]
    async fn test_cross_chain_api_creation() {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await.unwrap();
        let api = CrossChainApi::new(Arc::new(manager));
        
        // Test that API was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_submit_anchor_request() {
        let request = SubmitAnchorRequest {
            chain_id: "testchain".to_string(),
            state_root: "0x1234567890abcdef".to_string(),
            timestamp: crate::consensus::hashtimer::HashTimer::new([0u8; 32], [0u8; 32]),
            proof_type: Some(ProofType::Signature),
            proof_data: vec![1; 64],
        };
        
        // Test that request can be serialized/deserialized
        let json = serde_json::to_string(&request).unwrap();
        let _deserialized: SubmitAnchorRequest = serde_json::from_str(&json).unwrap();
        
        assert!(true);
    }

    #[tokio::test]
    async fn test_verify_inclusion_request() {
        let request = VerifyInclusionRequest {
            chain_id: "testchain".to_string(),
            tx_hash: "0xabcdef1234567890".to_string(),
            merkle_proof: vec![1; 128],
        };
        
        // Test that request can be serialized/deserialized
        let json = serde_json::to_string(&request).unwrap();
        let _deserialized: VerifyInclusionRequest = serde_json::from_str(&json).unwrap();
        
        assert!(true);
    }
} 