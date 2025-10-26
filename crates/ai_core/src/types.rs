//! Common types for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Model hash
    pub hash: String,
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model ID
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
    /// Model architecture
    pub architecture: String,
    /// Input shape
    pub input_shape: Vec<usize>,
    /// Output shape
    pub output_shape: Vec<usize>,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Parameter count
    pub parameter_count: u64,
}

/// Model input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    /// Input data
    pub data: Vec<u8>,
    /// Input type
    pub data_type: DataType,
    /// Input metadata
    pub metadata: HashMap<String, String>,
    /// Data type for compatibility
    pub dtype: DataType,
    /// Input shape
    pub shape: Vec<usize>,
}

/// Model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    /// Output data
    pub data: Vec<u8>,
    /// Output type
    pub data_type: DataType,
    /// Output metadata
    pub metadata: HashMap<String, String>,
    /// Confidence score
    pub confidence: f64,
    /// Output shape
    pub shape: Vec<usize>,
    /// Data type for compatibility
    pub dtype: DataType,
}

/// Data type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy)]
pub enum DataType {
    /// Text data
    Text,
    /// Binary data
    Binary,
    /// JSON data
    Json,
    /// Numeric data
    Numeric,
    /// Image data
    Image,
    /// Audio data
    Audio,
    /// Video data
    Video,
    /// Float32 data
    Float32,
    /// Float64 data
    Float64,
    /// Int8 data
    Int8,
    /// Int16 data
    Int16,
    /// Int32 data
    Int32,
    /// Int64 data
    Int64,
    /// UInt8 data
    UInt8,
    /// UInt16 data
    UInt16,
    /// UInt32 data
    UInt32,
    /// UInt64 data
    UInt64,
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
    /// Random seed for deterministic execution
    pub seed: Option<u64>,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Result data
    pub data: Vec<u8>,
    /// Result type
    pub data_type: DataType,
    /// Execution time (microseconds)
    pub execution_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Result metadata
    pub metadata: HashMap<String, String>,
    /// Model output
    pub output: ModelOutput,
    /// Execution context
    pub context: ExecutionContext,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution ID
    pub execution_id: String,
    /// Model version
    pub model_version: String,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Execution timestamp
    pub timestamp: u64,
}
