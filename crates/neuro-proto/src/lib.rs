// Mock implementation for now - we'll add real protobuf later
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostJobRequest {
    pub model_ref: Vec<u8>,
    pub input_commit: Vec<u8>,
    pub max_latency_ms: u32,
    pub region: String,
    pub max_price_ipn: u64,
    pub escrow_ipn: u64,
    pub privacy: PrivacyMode,
    pub bid_window_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostJobResponse {
    pub job_id: Vec<u8>,
    pub success: bool,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceBidRequest {
    pub job_id: Vec<u8>,
    pub executor_id: Vec<u8>,
    pub price_ipn: u64,
    pub est_latency_ms: u32,
    pub tee: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceBidResponse {
    pub success: bool,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPoIRequest {
    pub job_id: Vec<u8>,
    pub model_ref: Vec<u8>,
    pub output_commit: Vec<u8>,
    pub runtime_ms: u32,
    pub attestation: Vec<u8>,
    pub zk_proof: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPoIResponse {
    pub success: bool,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyMode {
    OPEN = 0,
    TEE = 1,
    ZK = 2,
}
