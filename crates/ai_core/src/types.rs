//! Common types for AI Core
//!
//! Defines model identifiers, metadata, input/output formats, execution
//! contexts, and deterministic inference metadata used across the AI Core.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for each model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelId {
    /// Model name
    pub name: String,
    /// Model version (semantic version or build hash)
    pub version: String,
    /// Model hash (Blake3 or SHA-256)
    pub hash: String,
}

impl ModelId {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        hash: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            hash: hash.into(),
        }
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
    /// Human-readable model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Description or summary
    pub description: String,
    /// Author or maintainer
    pub author: String,
    /// License string (e.g., MIT, Apache-2.0)
    pub license: String,
    /// Tags for classification or discoverability
    pub tags: Vec<String>,
    /// Creation timestamp (UNIX microseconds)
    pub created_at: u64,
    /// Last update timestamp (UNIX microseconds)
    pub updated_at: u64,
    /// Model architecture (e.g., "gbdt", "transformer", etc.)
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

/// Supported data types for model I/O tensors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// UTF-8 text
    Text,
    /// Arbitrary binary blob
    Binary,
    /// Structured JSON
    Json,
    /// Generic numeric type (e.g., f32)
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
    /// Encoded image data
    Image,
    /// Encoded audio data
    Audio,
    /// Encoded video data
    Video,
}

impl DataType {
    /// Returns the nominal byte size for this type (approximation)
    pub fn size_bytes(&self) -> usize {
        match self {
            DataType::Text | DataType::Binary | DataType::Json | DataType::Numeric => 1,
            DataType::Int8 | DataType::UInt8 => 1,
            DataType::Int16 | DataType::UInt16 => 2,
            DataType::Int32 | DataType::UInt32 | DataType::Float32 => 4,
            DataType::Int64 | DataType::UInt64 | DataType::Float64 => 8,
            DataType::Image | DataType::Audio | DataType::Video => 1,
        }
    }
}

/// Model input structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    /// Raw input bytes
    pub data: Vec<u8>,
    /// Data type
    pub dtype: DataType,
    /// Input shape
    pub shape: Vec<usize>,
    /// Input metadata (feature names, scaling, etc.)
    pub metadata: HashMap<String, String>,
}

/// Model output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Raw output bytes
    pub data: Vec<u8>,
    /// Data type
    pub dtype: DataType,
    /// Output shape
    pub shape: Vec<usize>,
    /// Confidence or quality score (1.0 = deterministic exact match)
    #[cfg(feature = "deterministic_math")]
    pub confidence: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub confidence: f64,
    /// Deterministic execution metadata
    pub metadata: ExecutionMetadata,
}

/// Execution context (per model invocation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub id: String,
    /// Model being executed
    pub model_id: ModelId,
    /// Arbitrary metadata (node, region, etc.)
    pub metadata: HashMap<String, String>,
    /// Parameters (e.g., quantization mode)
    pub parameters: HashMap<String, String>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Optional deterministic random seed
    pub seed: Option<u64>,
}

/// Deterministic execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution unique ID
    pub execution_id: String,
    /// Model identifier
    pub model_id: String,
    /// Execution start time (UNIX μs)
    pub start_time: u64,
    /// Execution end time (UNIX μs)
    pub end_time: u64,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Memory used in bytes
    pub memory_usage: u64,
    /// CPU usage percentage (0-10000 basis points)
    #[cfg(feature = "deterministic_math")]
    pub cpu_usage: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub cpu_usage: f64,
    /// Whether execution succeeded
    pub success: bool,
    /// Optional error message
    pub error: Option<String>,
    /// Additional metadata (e.g., deterministic signature, node info)
    pub metadata: HashMap<String, String>,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Model output
    pub output: ModelOutput,
    /// Execution context
    pub context: ExecutionContext,
    /// Whether the run succeeded
    pub success: bool,
    /// Optional error message
    pub error: Option<String>,
    /// Additional metadata (node, timestamp, etc.)
    pub metadata: HashMap<String, String>,
    /// Total execution time (μs)
    pub execution_time_us: u64,
    /// Total memory usage (bytes)
    pub memory_usage: u64,
    /// Output data type
    pub data_type: DataType,
}
