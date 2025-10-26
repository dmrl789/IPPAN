//! Common types for AI Core

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model identifier
pub type ModelId = String;

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
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

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Context ID
    pub id: String,
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
}
