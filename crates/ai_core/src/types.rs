//! Common types for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelId {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model hash
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

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model identifier
    pub id: ModelId,
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model description
    pub description: String,
    /// Model author
    pub author: String,
    /// Model license
    pub license: String,
    /// Model tags
    pub tags: Vec<String>,
    /// Model creation time
    pub created_at: u64,
    /// Model last updated time
    pub updated_at: u64,
    /// Input shape
    pub input_shape: Vec<usize>,
    /// Output shape
    pub output_shape: Vec<usize>,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Number of parameters
    pub parameter_count: u64,
    /// Model architecture description
    pub architecture: String,
}

/// Model input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    /// Input data
    pub data: Vec<u8>,
    /// Input type (alias for compatibility)
    pub data_type: DataType,
    /// Input data type
    pub dtype: DataType,
    /// Input shape
    pub shape: Vec<usize>,
    /// Input metadata
    pub metadata: HashMap<String, String>,
}

/// Execution metadata for model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU cycles consumed
    pub cpu_cycles: u64,
    /// Execution hash for determinism verification
    pub execution_hash: String,
    /// Model version used
    pub model_version: String,
}

/// Model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Output data
    pub data: Vec<u8>,
    /// Output type (alias for compatibility)
    pub data_type: DataType,
    /// Output data type
    pub dtype: DataType,
    /// Output shape
    pub shape: Vec<usize>,
    /// Output metadata
    pub metadata: ExecutionMetadata,
    /// Confidence score
    pub confidence: f64,
}

/// Data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// Text data
    Text,
    /// Binary data
    Binary,
    /// JSON data
    Json,
    /// Numeric data
    Numeric,
    /// 32-bit integer
    Int32,
    /// 64-bit integer
    Int64,
    /// 32-bit float
    Float32,
    /// Image data
    Image,
    /// Audio data
    Audio,
    /// Video data
    Video,
}

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Context ID
    pub id: String,
    /// Model ID
    pub model_id: ModelId,
    /// Context metadata
    pub metadata: HashMap<String, String>,
    /// Execution parameters
    pub parameters: HashMap<String, String>,
    /// Execution timeout
    pub timeout_ms: u64,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Output from execution
    pub output: ModelOutput,
    /// Execution context
    pub context: ExecutionContext,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}
