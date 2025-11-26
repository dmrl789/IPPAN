use crate::server::{
    deny_request, guard_request, record_security_failure, record_security_success, ApiError,
    AppState, ValidatedJson,
};
use axum::extract::{ConnectInfo, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::Json;
use blake3::Hasher;
use ippan_l2_handle_registry::{Handle, HandleRegistryError};
use ippan_types::address::encode_address;
use ippan_types::round::RoundFinalizationRecord;
use ippan_types::RoundId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::warn;

const IIL_VERSION: &str = "0.1";
const IIL_CANONICALIZATION: &str = "json-c14n-v1";
pub const IIL_QUERY_DEFAULT_LIMIT: usize = 50;
pub const IIL_QUERY_MAX_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IilFinality {
    pub round_id: RoundId,
    pub round_hash: String,
    pub ippan_time_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IilIntegrity {
    pub record_hash: String,
    pub canonicalization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IilProofBundle {
    pub finality: IilFinality,
    pub integrity: IilIntegrity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPayload {
    pub handle: String,
    pub owner: String,
    pub status: String,
    pub metadata: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IilRecord {
    pub hashid: String,
    pub kind: String,
    pub payload: IdentityPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IilProofedRecord {
    pub record: IilRecord,
    pub proof: IilProofBundle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct IilStatusResponse {
    pub iil_version: &'static str,
    pub finality: IilFinality,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct IilResolveResponse {
    pub iil_version: &'static str,
    pub finality: IilFinality,
    pub record: IilRecord,
    pub proof: IilProofBundle,
}

#[derive(Debug, Serialize)]
pub struct IilGetResponse {
    pub iil_version: &'static str,
    pub finality: IilFinality,
    pub record: IilRecord,
    pub proof: IilProofBundle,
}

#[derive(Debug, Serialize)]
pub struct IilQueryResponse {
    pub iil_version: &'static str,
    pub finality: IilFinality,
    pub results: Vec<IilProofedRecord>,
}

#[derive(Debug, Deserialize)]
pub struct IilQueryRequest {
    #[serde(default)]
    pub handles: Vec<String>,
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum IilError {
    #[error("invalid handle")]
    InvalidHandle,
    #[error("invalid hashid")]
    InvalidHashId,
    #[error("not implemented in v0.1")]
    NotImplemented,
    #[error("record not found")]
    NotFound,
    #[error("canonicalization failed: {0}")]
    Canonicalization(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("registry error: {0}")]
    Registry(String),
}

pub async fn handle_iil_status(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<IilStatusResponse>, (StatusCode, Json<ApiError>)> {
    const ENDPOINT: &str = "/iil/status";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let finality = latest_finality(&state);
    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(IilStatusResponse {
        iil_version: IIL_VERSION,
        finality,
        status: "ok",
    }))
}

pub async fn handle_iil_resolve(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(raw_handle): AxumPath<String>,
) -> Result<Json<IilResolveResponse>, (StatusCode, Json<ApiError>)> {
    const ENDPOINT: &str = "/iil/resolve";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    let handle = match validate_handle(&raw_handle) {
        Ok(handle) => handle,
        Err(err) => return Err(map_iil_error(&state, &addr, ENDPOINT, err).await),
    };

    let metadata = match state.handle_registry.get_metadata(&handle) {
        Ok(metadata) => metadata,
        Err(err) => return Err(map_registry_error(&state, &addr, ENDPOINT, err).await),
    };

    let finality = latest_finality(&state);
    let record = identity_record(&handle, metadata, &finality)?;
    record_security_success(&state, &addr, ENDPOINT).await;

    Ok(Json(IilResolveResponse {
        iil_version: IIL_VERSION,
        finality: finality.clone(),
        record: record.record,
        proof: record.proof,
    }))
}

pub async fn handle_iil_get(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumPath(hashid): AxumPath<String>,
) -> Result<Json<IilGetResponse>, (StatusCode, Json<ApiError>)> {
    const ENDPOINT: &str = "/iil/get";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    if !is_valid_hashid(&hashid) {
        return Err(map_iil_error(&state, &addr, ENDPOINT, IilError::InvalidHashId).await);
    }

    let finality = latest_finality(&state);
    let mut found: Option<IilProofedRecord> = None;
    for (handle, metadata) in state.handle_registry.snapshot() {
        let proofed = identity_record(&handle, metadata, &finality)?;
        if proofed.record.hashid == hashid {
            found = Some(proofed);
            break;
        }
    }

    if let Some(record) = found {
        record_security_success(&state, &addr, ENDPOINT).await;
        return Ok(Json(IilGetResponse {
            iil_version: IIL_VERSION,
            finality,
            record: record.record,
            proof: record.proof,
        }));
    }

    Err(map_iil_error(&state, &addr, ENDPOINT, IilError::NotFound).await)
}

pub async fn handle_iil_query(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ValidatedJson(request): ValidatedJson<IilQueryRequest>,
) -> Result<Json<IilQueryResponse>, (StatusCode, Json<ApiError>)> {
    const ENDPOINT: &str = "/iil/query";
    if let Err(err) = guard_request(&state, &addr, ENDPOINT).await {
        let (status, message) = deny_request(&state, &addr, ENDPOINT, err).await;
        return Err((status, Json(ApiError::new("security_error", message))));
    }

    if request
        .kinds
        .iter()
        .any(|kind| kind.eq_ignore_ascii_case("file") || kind.eq_ignore_ascii_case("service"))
    {
        return Err(map_iil_error(&state, &addr, ENDPOINT, IilError::NotImplemented).await);
    }

    let limit = request
        .limit
        .unwrap_or(IIL_QUERY_DEFAULT_LIMIT)
        .min(IIL_QUERY_MAX_LIMIT);

    let handles_filter: Option<Vec<Handle>> = if request.handles.is_empty() {
        None
    } else {
        let mut parsed = Vec::new();
        for raw in &request.handles {
            match validate_handle(raw) {
                Ok(handle) => parsed.push(handle),
                Err(err) => return Err(map_iil_error(&state, &addr, ENDPOINT, err).await),
            }
        }
        Some(parsed)
    };

    let finality = latest_finality(&state);
    let mut results: Vec<IilProofedRecord> = Vec::new();
    for (handle, metadata) in state.handle_registry.snapshot() {
        if let Some(filter) = &handles_filter {
            if !filter.iter().any(|candidate| candidate == &handle) {
                continue;
            }
        }
        let proofed = identity_record(&handle, metadata, &finality)?;
        results.push(proofed);
    }

    sort_results(&mut results);
    results.truncate(limit);

    record_security_success(&state, &addr, ENDPOINT).await;
    Ok(Json(IilQueryResponse {
        iil_version: IIL_VERSION,
        finality,
        results,
    }))
}

fn sort_results(results: &mut [IilProofedRecord]) {
    results.sort_by(|left, right| {
        let left_score = left.score.unwrap_or(0);
        let right_score = right.score.unwrap_or(0);
        right_score
            .cmp(&left_score)
            .then_with(|| left.record.hashid.cmp(&right.record.hashid))
    });
}

fn identity_record(
    handle: &Handle,
    metadata: ippan_l2_handle_registry::HandleMetadata,
    finality: &IilFinality,
) -> Result<IilProofedRecord, (StatusCode, Json<ApiError>)> {
    let payload = IdentityPayload {
        handle: handle.as_str().to_string(),
        owner: encode_address(metadata.owner.as_bytes()),
        status: format_handle_status(&metadata.status),
        metadata: metadata
            .metadata
            .into_iter()
            .collect::<BTreeMap<String, String>>(),
        expires_at: if metadata.expires_at == 0 {
            None
        } else {
            Some(metadata.expires_at)
        },
    };

    let content = IilRecordContent {
        kind: "identity".to_string(),
        payload: payload.clone(),
    };
    let record_hash = match canonical_record_hash(&content) {
        Ok(hash) => hash,
        Err(err) => return Err(map_internal_error(err)),
    };

    let record = IilRecord {
        hashid: record_hash.clone(),
        kind: content.kind,
        payload,
    };

    let proof = IilProofBundle {
        finality: finality.clone(),
        integrity: IilIntegrity {
            record_hash,
            canonicalization: IIL_CANONICALIZATION.to_string(),
        },
    };

    Ok(IilProofedRecord {
        record,
        proof,
        score: Some(deterministic_score(handle)),
    })
}

fn deterministic_score(handle: &Handle) -> i64 {
    (200i64).saturating_sub(handle.as_str().len() as i64)
}

fn validate_handle(raw: &str) -> Result<Handle, IilError> {
    let decoded = raw.trim();
    if decoded.len() > 64 {
        return Err(IilError::InvalidHandle);
    }
    if !decoded.starts_with('@') || !decoded.ends_with(".ipn") {
        return Err(IilError::InvalidHandle);
    }
    let handle = Handle::new(decoded.to_string());
    if !handle.is_valid() {
        return Err(IilError::InvalidHandle);
    }
    Ok(handle)
}

fn is_valid_hashid(raw: &str) -> bool {
    raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit())
}

fn latest_finality(state: &AppState) -> IilFinality {
    let fallback = IilFinality {
        round_id: 0,
        round_hash: "0".repeat(64),
        ippan_time_us: 0,
    };

    match state.storage.get_latest_round_finalization() {
        Ok(Some(record)) => finality_from_record(record),
        Ok(None) => fallback,
        Err(err) => {
            warn!("Failed to fetch finality for IIL: {}", err);
            fallback
        }
    }
}

fn finality_from_record(record: RoundFinalizationRecord) -> IilFinality {
    IilFinality {
        round_id: record.round,
        round_hash: hex::encode(record.state_root),
        ippan_time_us: record.window.end_us.0,
    }
}

fn format_handle_status(status: &ippan_l2_handle_registry::HandleStatus) -> String {
    match status {
        ippan_l2_handle_registry::HandleStatus::Active => "active",
        ippan_l2_handle_registry::HandleStatus::Suspended => "suspended",
        ippan_l2_handle_registry::HandleStatus::Expired => "expired",
        ippan_l2_handle_registry::HandleStatus::Transferred => "transferred",
    }
    .to_string()
}

fn canonical_record_hash(content: &IilRecordContent) -> Result<String, IilError> {
    let value =
        serde_json::to_value(content).map_err(|err| IilError::Serialization(err.to_string()))?;
    let canonical = canonicalize_value(&value)?;
    let json = serde_json::to_string(&canonical)
        .map_err(|err| IilError::Serialization(err.to_string()))?;
    let mut hasher = Hasher::new();
    hasher.update(json.as_bytes());
    Ok(hex::encode(hasher.finalize().as_bytes()))
}

fn canonicalize_value(value: &Value) -> Result<Value, IilError> {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (key, val) in map {
                sorted.insert(key.clone(), canonicalize_value(val)?);
            }
            Ok(Value::Object(sorted.into_iter().collect()))
        }
        Value::Array(values) => {
            let mut normalized = Vec::new();
            for v in values {
                normalized.push(canonicalize_value(v)?);
            }
            Ok(Value::Array(normalized))
        }
        Value::Number(num) if num.is_f64() => Err(IilError::Canonicalization(
            "floating point values not supported".to_string(),
        )),
        other => Ok(other.clone()),
    }
}

fn map_internal_error(err: IilError) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError::new("internal_error", err.to_string())),
    )
}

async fn map_registry_error(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    err: HandleRegistryError,
) -> (StatusCode, Json<ApiError>) {
    let (status, code, message) = match err {
        HandleRegistryError::HandleNotFound { .. } => {
            (StatusCode::NOT_FOUND, "handle_not_found", err.to_string())
        }
        HandleRegistryError::HandleExpired { .. } => {
            (StatusCode::NOT_FOUND, "handle_expired", err.to_string())
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            err.to_string(),
        ),
    };

    if status.is_server_error() {
        record_security_failure(state, addr, endpoint, &message).await;
    } else {
        record_security_success(state, addr, endpoint).await;
    }

    (status, Json(ApiError::new(code, message)))
}

async fn map_iil_error(
    state: &Arc<AppState>,
    addr: &SocketAddr,
    endpoint: &str,
    err: IilError,
) -> (StatusCode, Json<ApiError>) {
    let (status, code, message) = match err {
        IilError::InvalidHandle => (StatusCode::BAD_REQUEST, "invalid_handle", err.to_string()),
        IilError::InvalidHashId => (StatusCode::BAD_REQUEST, "invalid_hashid", err.to_string()),
        IilError::NotImplemented => (
            StatusCode::NOT_IMPLEMENTED,
            "not_implemented",
            err.to_string(),
        ),
        IilError::NotFound => (StatusCode::NOT_FOUND, "record_not_found", err.to_string()),
        IilError::Canonicalization(_) | IilError::Serialization(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            err.to_string(),
        ),
        IilError::Registry(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            err.to_string(),
        ),
    };

    if status.is_server_error() {
        record_security_failure(state, addr, endpoint, &message).await;
    } else {
        record_security_success(state, addr, endpoint).await;
    }

    (status, Json(ApiError::new(code, message)))
}

#[derive(Debug, Clone, Serialize)]
struct IilRecordContent {
    kind: String,
    payload: IdentityPayload,
}
