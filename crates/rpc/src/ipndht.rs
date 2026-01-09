//! IPNDHT RPC endpoints for handle and file discovery.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::server::{ApiError, AppState};

/// Query parameters for paginated IPNDHT endpoints.
#[derive(Debug, Deserialize, Default)]
pub struct IpndhtQueryParams {
    /// Maximum number of items to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Summary response for /ipndht/summary
#[derive(Debug, Serialize)]
pub struct IpndhtSummaryResponse {
    pub ok: bool,
    /// Number of cached handles.
    pub handles: usize,
    /// Number of cached file descriptors.
    pub files: usize,
    /// Number of DHT provider records (0 if DHT not enabled).
    pub providers: usize,
    /// Number of connected DHT peers.
    pub dht_peers: usize,
    /// Whether the DHT service is enabled.
    pub dht_enabled: bool,
    /// Timestamp (milliseconds since epoch).
    pub ts: u64,
}

/// Handle item in list response.
#[derive(Debug, Serialize)]
pub struct HandleItem {
    pub handle: String,
    pub owner: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

/// Response for /ipndht/handles
#[derive(Debug, Serialize)]
pub struct IpndhtHandlesResponse {
    pub ok: bool,
    pub items: Vec<HandleItem>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// File item in list response.
#[derive(Debug, Serialize)]
pub struct FileItem {
    pub id: String,
    pub content_hash: String,
    pub owner: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub created_at_us: u64,
}

/// Response for /ipndht/files
#[derive(Debug, Serialize)]
pub struct IpndhtFilesResponse {
    pub ok: bool,
    pub items: Vec<FileItem>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// GET /ipndht/summary - Get IPNDHT service summary statistics
pub async fn handle_ipndht_summary(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<IpndhtSummaryResponse>, (StatusCode, Json<ApiError>)> {
    // Security check
    if let Err(err) = guard_ipndht_request(&state, &addr, "/ipndht/summary").await {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new("security_error", err.to_string())),
        ));
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Get handle count from registry
    let handles = state.handle_registry.count().unwrap_or(0);

    // Get file count from storage
    let files = if let Some(file_storage) = &state.file_storage {
        file_storage.count().unwrap_or(0)
    } else {
        0
    };

    // Check if DHT is enabled
    let dht_enabled = state.ipn_dht.is_some();

    // Get DHT peer count (from P2P network if available)
    let dht_peers = state.peer_count.load(std::sync::atomic::Ordering::Relaxed);

    Ok(Json(IpndhtSummaryResponse {
        ok: true,
        handles,
        files,
        providers: 0, // TODO: Track provider count if needed
        dht_peers,
        dht_enabled,
        ts,
    }))
}

/// GET /ipndht/handles - List cached handles
pub async fn handle_ipndht_handles(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<IpndhtQueryParams>,
) -> Result<Json<IpndhtHandlesResponse>, (StatusCode, Json<ApiError>)> {
    // Security check
    if let Err(err) = guard_ipndht_request(&state, &addr, "/ipndht/handles").await {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new("security_error", err.to_string())),
        ));
    }

    let limit = params.limit.min(200); // Cap at 200

    // Get handles from registry
    let (items, total) = match state.handle_registry.list(limit) {
        Ok((list_items, count)) => {
            let items: Vec<HandleItem> = list_items
                .into_iter()
                .map(|item| HandleItem {
                    handle: item.handle,
                    owner: item.owner,
                    expires_at: item.expires_at,
                    updated_at: item.updated_at,
                })
                .collect();
            (items, count)
        }
        Err(e) => {
            warn!("Failed to list handles: {}", e);
            (Vec::new(), 0)
        }
    };

    Ok(Json(IpndhtHandlesResponse {
        ok: true,
        items,
        total,
        next_cursor: None, // TODO: Implement pagination cursor
    }))
}

/// GET /ipndht/files - List cached file descriptors
pub async fn handle_ipndht_files(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<IpndhtQueryParams>,
) -> Result<Json<IpndhtFilesResponse>, (StatusCode, Json<ApiError>)> {
    // Security check
    if let Err(err) = guard_ipndht_request(&state, &addr, "/ipndht/files").await {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new("security_error", err.to_string())),
        ));
    }

    let limit = params.limit.min(200); // Cap at 200

    // Get files from storage
    let (items, total) = if let Some(file_storage) = &state.file_storage {
        let count = file_storage.count().unwrap_or(0) as usize;
        match file_storage.list(0, limit) {
            Ok(files) => {
                let items: Vec<FileItem> = files
                    .into_iter()
                    .map(|desc| FileItem {
                        id: desc.id.to_hex(),
                        content_hash: desc.content_hash.to_hex(),
                        owner: ippan_types::address::encode_address(&desc.owner),
                        size_bytes: desc.size_bytes,
                        mime_type: desc.mime_type,
                        created_at_us: desc.created_at_us,
                    })
                    .collect();
                (items, count)
            }
            Err(e) => {
                warn!("Failed to list files: {}", e);
                (Vec::new(), 0)
            }
        }
    } else {
        (Vec::new(), 0)
    };

    Ok(Json(IpndhtFilesResponse {
        ok: true,
        items,
        total,
        next_cursor: None, // TODO: Implement pagination cursor
    }))
}

/// Security guard for IPNDHT endpoints
async fn guard_ipndht_request(
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

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 50);
    }

    #[test]
    fn test_query_params_defaults() {
        let params: IpndhtQueryParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.limit, 50);
    }

    #[test]
    fn test_query_params_custom_limit() {
        let params: IpndhtQueryParams = serde_json::from_str(r#"{"limit": 100}"#).unwrap();
        assert_eq!(params.limit, 100);
    }
}
