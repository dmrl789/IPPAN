//! Common types for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for each model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelId {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model hash (Blake3 or SHA-256)
    pub hash: String,
}

impl ModelId {
    pub fn new(name: String, version: String, hash: String) -> Self {
        Self { name, version, hash }
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.name, self.version, self.hash)
    }
}

/// Model metadata (structural and descriptive info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model identifier
    pub id: ModelId,
    /// Human-readable name
    pub name: String,
    /// Model version (semver or hash)
    pub version: String,
    /// Model description
    pub description: String,
    /// Author or maintainer
    pub author: String,
    /// License string (e.g., MIT, Apache-2.0)
    pub license: String,
    /// Tags for discoverability
    pub tags: Vec<String>,
    /// Creation timestamp (UNIX microseconds)
    pub created_at: u64,
    /// Last update timestamp (UNIX microseconds)
    pub updated_at: u64,
    /// Model architecture (e.g., "gbdt", "transformer", "cnn")
    pub architecture: String,
    /// Input tensor shape
    pub input_shape: Vec<usize>,
    /// Output tensor shape
    pub output_shape: Vec<usize>,
    /// Model binary size in bytes
    pub size_bytes: u64,
    /// Number of parameters
    pub parameter_count: u64,
}

/// Model input structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    /// Raw input data (bytes)
    pub data: Vec<u8>,
    /// Declared data type
    pub dtype: DataType,
    /// Shape of the input tensor
    pub shape: Vec<usize>,
    /// Arbitrary metadata (feature names, scaling, etc.)
    pub metadata: HashMap<String, String>,
}

/// Model output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Raw output bytes
    pub data: Vec<u8>,
    /// Output data type
    pub dtype: DataType,
    /// Output shape
    pub shape: Vec<usize>,
    /// Confidence or quality score (1.0 = deterministic exact)
    pub confidence: f64,
    /// Deterministic execution metadata
    pub metadata: ExecutionMetadata,
}

/// Execution metadata (used for deterministic verification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution duration in microseconds
    pub execution_time_us: u64,
    /// Total memory used (bytes)
    pub memory_usage_bytes: u64,
    /// CPU cycles estimated for execution
    pub cpu_cycles: u64,
    /// Deterministic execution hash (Blake3)
    pub execution_hash: String,
    /// Model version used for inference
    pub model_version: String,
}

/// Supported data types for AI Core tensors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// UTF-8 text
    Text,
    /// Arbitrary binary blob
    Binary,
    /// Structured JSON
    Json,
    /// Generic numeric type
    Numeric,
    /// 32-bit integer
    Int32,
    /// 64-bit integer
    Int64,
    /// 32-bit float
    Float32,
    /// 64-bit float
    Float64,
    /// 8-bit signed integer
    Int8,
    /// 16-bit signed integer
    Int16,
    /// 8-bit unsigned integer
    UInt8,
    /// 16-bit unsigned integer
    UInt16,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit unsigned integer
    UInt64,
    /// Image (encoded)
    Image,
    /// Audio (encoded)
    Audio,
    /// Video (encoded)
    Video,
}

/// Execution context (per model call)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub id: String,
    /// Model ID to execute
    pub model_id: ModelId,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
    /// Parameters (e.g., temperature, quantization)
    pub parameters: HashMap<String, String>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Optional deterministic random seed
    pub seed: Option<u64>,
}

/// Execution result returned to caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Model output
    pub output: ModelOutput,
    /// Execution context
    pub context: ExecutionContext,
    /// Whether execution succeeded
    pub success: bool,
    /// Optional error message
    pub error: Option<String>,
    /// Optional additional metadata (e.g., host, timestamp)
    pub metadata: HashMap<String, String>,
    /// Execution time (duplicate for summary)
    pub execution_time_us: u64,
    /// Total memory usage
    pub memory_usage: u64,
    /// Data type for compatibility with legacy outputs
    pub data_type: DataType,
}
