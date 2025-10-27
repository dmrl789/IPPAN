//! Common types for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model identifier
pub type ModelId = String;

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
    /// Model hash
    pub hash: String,
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
    /// Input shape
    pub shape: Vec<usize>,
    /// Data type for size calculation
    pub dtype: DataType,
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
}

/// Data type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl DataType {
    /// Get the size in bytes for this data type
    pub fn size_bytes(&self) -> usize {
        match self {
            DataType::Text => 1, // UTF-8 character
            DataType::Binary => 1,
            DataType::Json => 1,
            DataType::Numeric => 4, // Assume float32
            DataType::Image => 1,
            DataType::Audio => 1,
            DataType::Video => 1,
        }
    }
}

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Context ID
    pub id: String,
    /// Model ID
    pub model_id: String,
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
    /// Output data
    pub output: ModelOutput,
    /// Execution context
    pub context: ExecutionContext,
}

/// Execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Execution ID
    pub execution_id: String,
    /// Model ID
    pub model_id: String,
    /// Execution start time
    pub start_time: u64,
    /// Execution end time
    pub end_time: u64,
    /// Execution duration (microseconds)
    pub duration_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}
