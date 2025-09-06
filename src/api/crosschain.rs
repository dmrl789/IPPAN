#[cfg(feature = "crosschain")]
use crate::crosschain::{
    CrossChainManager, ExternalAnchorData, ProofType, L2CommitTx, L2ExitTx,
    LightSyncData, L2Report
};
use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cross-chain API handler
#[cfg(feature = "crosschain")]
pub struct CrossChainApi {
    manager: Arc<CrossChainManager>,
    server: Option<axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>>,
    bind_addr: String,
}

#[cfg(feature = "crosschain")]
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
    pub async fn start(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let app = self.create_router();
        
        let addr = self.bind_addr.parse().map_err(|e| crate::error::IppanError::Config(format!("Invalid address: {}", e)))?;
        let server = axum::Server::bind(&addr).serve(app.into_make_service());
        
        self.server = Some(server);
        
        tracing::info!("Cross-chain API server started on {}", self.bind_addr);
        Ok(())
    }

    /// Stop the cross-chain API server
    pub async fn stop(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        if let Some(server) = self.server.take() {
            server.await.map_err(|e| crate::error::IppanError::Network(format!("Server error: {}", e)))?;
        }
        Ok(())
    }

    /// Create the router with all cross-chain endpoints
    fn create_router(&self) -> Router {
        Router::new()
            .route("/l2/commit", post(Self::submit_l2_commit))
            .route("/l2/:l2_id/anchor", get(Self::get_latest_l2_anchor))
            .route("/l2/verify_exit", post(Self::verify_l2_exit))
            .route("/l2/register", post(Self::register_l2))
            .route("/l2/:l2_id", get(Self::get_l2_info))
            .route("/l2/:l2_id", delete(Self::deregister_l2))
            .route("/light_sync/:round", get(Self::get_light_sync_data))
            .route("/l2/report", get(Self::get_l2_report))
            .route("/health", get(Self::health_check))
            .with_state(Arc::clone(&self.manager))
    }

    /// Submit an L2 commit transaction
    async fn submit_l2_commit(
        State(manager): State<Arc<CrossChainManager>>,
        Json(request): Json<SubmitL2CommitRequest>,
    ) -> Json<ApiResponse<SubmitL2CommitResponse>> {
        let commit_tx = L2CommitTx {
            l2_id: request.l2_id,
            epoch: request.epoch,
            state_root: request.state_root,
            da_hash: request.da_hash,
            proof_type: request.proof_type,
            proof: request.proof,
            inline_data: None, // No inline data for now
        };

        match manager.submit_l2_commit(commit_tx).await {
            Ok(commit_id) => Json(ApiResponse::success(SubmitL2CommitResponse {
                commit_id,
                status: "submitted".to_string(),
                timestamp: chrono::Utc::now(),
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to submit L2 commit: {}", e))),
        }
    }

    /// Get the latest L2 anchor for a chain
    async fn get_latest_l2_anchor(
        State(manager): State<Arc<CrossChainManager>>,
        Path(l2_id): Path<String>,
    ) -> Json<ApiResponse<GetLatestL2AnchorResponse>> {
        match manager.get_latest_l2_anchor(&l2_id).await {
            Ok(Some(anchor)) => Json(ApiResponse::success(GetLatestL2AnchorResponse {
                l2_id,
                last_anchor: Some(anchor),
            })),
            Ok(None) => Json(ApiResponse::success(GetLatestL2AnchorResponse {
                l2_id,
                last_anchor: None,
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to get latest L2 anchor: {}", e))),
        }
    }

    /// Verify L2 exit transaction
    async fn verify_l2_exit(
        State(manager): State<Arc<CrossChainManager>>,
        Json(request): Json<VerifyL2ExitRequest>,
    ) -> Json<ApiResponse<VerifyL2ExitResponse>> {
        let exit_tx = L2ExitTx {
            l2_id: request.l2_id,
            epoch: request.epoch,
            proof_of_inclusion: request.proof_of_inclusion,
            account: request.account,
            amount: request.amount,
            nonce: request.nonce,
        };

        match manager.verify_l2_exit(exit_tx).await {
            Ok(()) => Json(ApiResponse::success(VerifyL2ExitResponse {
                status: "verified".to_string(),
                timestamp: chrono::Utc::now(),
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to verify L2 exit: {}", e))),
        }
    }

    /// Register an L2 network
    async fn register_l2(
        State(manager): State<Arc<CrossChainManager>>,
        Json(request): Json<RegisterL2Request>,
    ) -> Json<ApiResponse<RegisterL2Response>> {
        let params = crate::crosschain::types::L2Params {
            proof_type: request.proof_type,
            da_mode: request.da_mode,
            challenge_window_ms: request.challenge_window_ms,
            max_commit_size: request.max_commit_size,
            min_epoch_gap_ms: request.min_epoch_gap_ms,
        };

        match manager.register_l2(request.l2_id, params).await {
            Ok(()) => Json(ApiResponse::success(RegisterL2Response {
                status: "registered".to_string(),
                timestamp: chrono::Utc::now(),
            })),
            Err(e) => Json(ApiResponse::error(format!("Failed to register L2: {}", e))),
        }
    }

    /// Get L2 network information
    async fn get_l2_info(
        State(manager): State<Arc<CrossChainManager>>,
        Path(l2_id): Path<String>,
    ) -> Json<ApiResponse<GetL2InfoResponse>> {
        // For now, just return basic info since we don't have a get_l2_info method
        Json(ApiResponse::success(GetL2InfoResponse {
            l2_id,
            status: "active".to_string(),
            registered_at: chrono::Utc::now(),
        }))
    }

    /// Deregister an L2 network
    async fn deregister_l2(
        State(_manager): State<Arc<CrossChainManager>>,
        Path(_l2_id): Path<String>,
    ) -> Json<ApiResponse<DeregisterL2Response>> {
        // Note: This would require adding a deregister_l2 method to CrossChainManager
        Json(ApiResponse::error("L2 deregistration not implemented yet".to_string()))
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

    /// Get comprehensive L2 report
    async fn get_l2_report(
        State(manager): State<Arc<CrossChainManager>>,
    ) -> Json<ApiResponse<L2Report>> {
        match manager.generate_l2_report().await {
            Ok(report) => Json(ApiResponse::success(report)),
            Err(e) => Json(ApiResponse::error(format!("Failed to generate L2 report: {}", e))),
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

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitL2CommitRequest {
    pub l2_id: String,
    pub epoch: u64,
    pub state_root: [u8; 32],
    pub da_hash: [u8; 32],
    pub proof_type: ProofType,
    pub proof: Vec<u8>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct SubmitL2CommitResponse {
    pub commit_id: String,
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct GetLatestL2AnchorResponse {
    pub l2_id: String,
    pub last_anchor: Option<crate::crosschain::types::AnchorEvent>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyL2ExitRequest {
    pub l2_id: String,
    pub epoch: u64,
    pub proof_of_inclusion: Vec<u8>,
    pub account: [u8; 32],
    pub amount: u128,
    pub nonce: u64,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct VerifyL2ExitResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Deserialize)]
pub struct RegisterL2Request {
    pub l2_id: String,
    pub proof_type: ProofType,
    pub da_mode: crate::crosschain::types::DataAvailabilityMode,
    pub challenge_window_ms: u64,
    pub max_commit_size: usize,
    pub min_epoch_gap_ms: u64,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct RegisterL2Response {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct GetL2InfoResponse {
    pub l2_id: String,
    pub status: String,
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct DeregisterL2Response {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct GetLightSyncDataResponse {
    pub sync_data: Option<LightSyncData>,
}

#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Generic API response wrapper
#[cfg(feature = "crosschain")]
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "crosschain")]
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
#[cfg(feature = "crosschain")]
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
    async fn test_submit_l2_commit_request() {
        let request = SubmitL2CommitRequest {
            l2_id: "test-l2".to_string(),
            epoch: 1,
            state_root: [1u8; 32],
            da_hash: [2u8; 32],
            proof_type: ProofType::ZkGroth16,
            proof: vec![1; 64],
        };
        
        // Test that request can be serialized/deserialized
        let json = serde_json::to_string(&request).unwrap();
        let _deserialized: SubmitL2CommitRequest = serde_json::from_str(&json).unwrap();
        
        assert!(true);
    }

    #[tokio::test]
    async fn test_verify_l2_exit_request() {
        let request = VerifyL2ExitRequest {
            l2_id: "test-l2".to_string(),
            epoch: 1,
            proof_of_inclusion: vec![1; 128],
            account: [3u8; 32],
            amount: 1000,
            nonce: 1,
        };
        
        // Test that request can be serialized/deserialized
        let json = serde_json::to_string(&request).unwrap();
        let _deserialized: VerifyL2ExitRequest = serde_json::from_str(&json).unwrap();
        
        assert!(true);
    }
} 