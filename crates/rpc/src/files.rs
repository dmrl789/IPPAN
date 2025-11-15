//! File descriptor RPC endpoints for IPNDHT file publishing and lookup.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::Json;
use ippan_files::{
    ContentHash, FileDhtService, FileDescriptor, FileId, FileStorage, StubFileDhtService,
};
use ippan_types::address::{decode_address, encode_address};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::server::{ApiError, AppState};

/// Request to publish a file descriptor.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishFileRequest {
    /// Owner address (base58check or hex).
    pub owner: String,
    
    /// Content hash (hex, 64 characters).
    pub content_hash: String,
    
    /// File size in bytes.
    pub size_bytes: u64,
    
    /// Optional MIME type.
    #[serde(default)]
    pub mime_type: Option<String>,
    
    /// Optional tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Response from publishing a file descriptor.
#[derive(Debug, Serialize)]
pub struct PublishFileResponse {
    /// Generated file ID.
    pub id: String,
    
    /// Content hash (echoed back).
    pub content_hash: String,
    
    /// Owner address.
    pub owner: String,
    
    /// File size in bytes.
    pub size_bytes: u64,
    
    /// Creation timestamp (microseconds).
    pub created_at_us: u64,
    
    /// Optional MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// Optional tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    
    /// DHT publish status.
    pub dht_published: bool,
}

/// Response for file descriptor lookup.
#[derive(Debug, Serialize)]
pub struct FileDescriptorResponse {
    /// File ID.
    pub id: String,
    
    /// Content hash.
    pub content_hash: String,
    
    /// Owner address.
    pub owner: String,
    
    /// File size in bytes.
    pub size_bytes: u64,
    
    /// Creation timestamp (microseconds).
    pub created_at_us: u64,
    
    /// Optional MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// Optional tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl From<FileDescriptor> for FileDescriptorResponse {
    fn from(desc: FileDescriptor) -> Self {
        Self {
            id: desc.id.to_hex(),
            content_hash: desc.content_hash.to_hex(),
            owner: encode_address(&desc.owner),
            size_bytes: desc.size_bytes,
            created_at_us: desc.created_at_us,
            mime_type: desc.mime_type,
            tags: desc.tags,
        }
    }
}

/// POST /files/publish - Publish a file descriptor
pub async fn handle_publish_file(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<PublishFileRequest>,
) -> Result<Json<PublishFileResponse>, (StatusCode, Json<ApiError>)> {
    // Security check
    if let Err(err) = guard_file_request(&state, &addr, "/files/publish").await {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new("security_error", &err.to_string())),
        ));
    }
    
    // Parse owner address
    let owner_bytes = decode_address(&request.owner).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("invalid_owner", &format!("Invalid owner address: {}", e))),
        )
    })?;
    
    // Parse content hash
    let content_hash = ContentHash::from_hex(&request.content_hash).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("invalid_content_hash", &format!("Invalid content hash: {}", e))),
        )
    })?;
    
    // Validate size
    if request.size_bytes == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("invalid_size", "File size must be greater than zero")),
        ));
    }
    
    // Create file descriptor
    let descriptor = FileDescriptor::new(
        content_hash,
        owner_bytes,
        request.size_bytes,
        request.mime_type,
        request.tags,
    );
    
    // Validate descriptor
    if let Err(e) = descriptor.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("validation_failed", &e)),
        ));
    }
    
    // Store locally
    if let Some(file_storage) = &state.file_storage {
        file_storage.store(descriptor.clone()).map_err(|e| {
            warn!("Failed to store file descriptor: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("storage_error", "Failed to store file descriptor")),
            )
        })?;
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError::new("storage_unavailable", "File storage not configured")),
        ));
    }
    
    // Publish to DHT
    let dht_published = if let Some(dht_service) = &state.file_dht {
        match dht_service.publish_file(&descriptor) {
            Ok(result) => {
                debug!("DHT publish result: {:?}", result);
                result.success
            }
            Err(e) => {
                warn!("DHT publish failed (non-fatal): {}", e);
                false
            }
        }
    } else {
        false
    };
    
    // Return response
    Ok(Json(PublishFileResponse {
        id: descriptor.id.to_hex(),
        content_hash: descriptor.content_hash.to_hex(),
        owner: encode_address(&descriptor.owner),
        size_bytes: descriptor.size_bytes,
        created_at_us: descriptor.created_at_us,
        mime_type: descriptor.mime_type,
        tags: descriptor.tags,
        dht_published,
    }))
}

/// GET /files/{id} - Lookup a file descriptor by ID
pub async fn handle_get_file(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(id_hex): AxumPath<String>,
) -> Result<Json<FileDescriptorResponse>, (StatusCode, Json<ApiError>)> {
    // Security check
    if let Err(err) = guard_file_request(&state, &addr, "/files/{id}").await {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new("security_error", &err.to_string())),
        ));
    }
    
    // Parse file ID
    let file_id = FileId::from_hex(&id_hex).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("invalid_file_id", &format!("Invalid file ID: {}", e))),
        )
    })?;
    
    // Try local storage first
    if let Some(file_storage) = &state.file_storage {
        match file_storage.get(&file_id) {
            Ok(Some(descriptor)) => {
                return Ok(Json(FileDescriptorResponse::from(descriptor)));
            }
            Ok(None) => {
                // Not found locally, try DHT
            }
            Err(e) => {
                warn!("Storage error during lookup: {}", e);
            }
        }
    }
    
    // Try DHT lookup if local not found
    if let Some(dht_service) = &state.file_dht {
        match dht_service.find_file(&file_id) {
            Ok(result) => {
                if let Some(descriptor) = result.descriptor {
                    return Ok(Json(FileDescriptorResponse::from(descriptor)));
                }
            }
            Err(e) => {
                warn!("DHT lookup error: {}", e);
            }
        }
    }
    
    // Not found
    Err((
        StatusCode::NOT_FOUND,
        Json(ApiError::new("not_found", "File descriptor not found")),
    ))
}

/// Security guard for file endpoints
async fn guard_file_request(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
) -> anyhow::Result<()> {
    if let Some(security) = &state.security {
        security.check_request(addr.ip(), endpoint).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_files::MemoryFileStorage;
    
    #[test]
    fn test_publish_request_parsing() {
        let json = r#"{
            "owner": "ippan1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqwkl3a9",
            "content_hash": "0000000000000000000000000000000000000000000000000000000000000001",
            "size_bytes": 1024,
            "mime_type": "text/plain",
            "tags": ["test"]
        }"#;
        
        let req: PublishFileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.size_bytes, 1024);
        assert_eq!(req.mime_type, Some("text/plain".to_string()));
        assert_eq!(req.tags, vec!["test".to_string()]);
    }
    
    #[test]
    fn test_descriptor_response_conversion() {
        let content_hash = ContentHash::from_data(b"test");
        let owner = [1u8; 32];
        let desc = FileDescriptor::new(content_hash, owner, 100, Some("text/plain".to_string()), vec!["tag1".to_string()]);
        
        let response = FileDescriptorResponse::from(desc.clone());
        assert_eq!(response.id, desc.id.to_hex());
        assert_eq!(response.size_bytes, 100);
        assert_eq!(response.mime_type, Some("text/plain".to_string()));
        assert_eq!(response.tags, vec!["tag1".to_string()]);
    }
}
