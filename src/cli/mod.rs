//! Command-line interface for IPPAN
//!
//! Provides a comprehensive CLI for node management, wallet operations,
//! blockchain interaction, and system administration.

use crate::{Result, IppanError, TransactionHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

pub mod cli_manager;
pub mod node_commands;
pub mod wallet_commands;
pub mod blockchain_commands;
pub mod network_commands;
pub mod admin_commands;
pub mod interactive_shell;
pub mod command_parser;
pub mod output_formatter;

pub use cli_manager::CLIManager;
pub use node_commands::NodeCommands;
pub use wallet_commands::WalletCommands;
pub use blockchain_commands::BlockchainCommands;
pub use network_commands::NetworkCommands;
pub use admin_commands::AdminCommands;
pub use interactive_shell::InteractiveShell;
pub use command_parser::CommandParser;
pub use output_formatter::OutputFormatter;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIConfig {
    /// Enable interactive mode
    pub enable_interactive_mode: bool,
    /// Enable colored output
    pub enable_colored_output: bool,
    /// Enable verbose output
    pub enable_verbose_output: bool,
    /// Default output format
    pub default_output_format: OutputFormat,
    /// Command history size
    pub command_history_size: usize,
    /// Auto-completion enabled
    pub auto_completion_enabled: bool,
    /// Command timeout in seconds
    pub command_timeout_seconds: u64,
    /// Enable command logging
    pub enable_command_logging: bool,
    /// Log file path
    pub log_file_path: String,
    /// Enable command aliases
    pub enable_command_aliases: bool,
    /// Custom command aliases
    pub command_aliases: HashMap<String, String>,
}

impl Default for CLIConfig {
    fn default() -> Self {
        let mut command_aliases = HashMap::new();
        command_aliases.insert("ls".to_string(), "list".to_string());
        command_aliases.insert("ps".to_string(), "status".to_string());
        command_aliases.insert("q".to_string(), "quit".to_string());
        command_aliases.insert("h".to_string(), "help".to_string());
        
        Self {
            enable_interactive_mode: true,
            enable_colored_output: true,
            enable_verbose_output: false,
            default_output_format: OutputFormat::Table,
            command_history_size: 1000,
            auto_completion_enabled: true,
            command_timeout_seconds: 30,
            enable_command_logging: true,
            log_file_path: "cli.log".to_string(),
            enable_command_aliases: true,
            command_aliases,
        }
    }
}

/// Output format for CLI results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum OutputFormat {
    /// Table format
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// CSV format
    Csv,
    /// Plain text format
    Plain,
    /// Pretty format
    Pretty,
}

/// CLI command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIResult {
    /// Command success
    pub success: bool,
    /// Result data
    pub data: Option<serde_json::Value>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Command name
    pub command_name: String,
    /// Command arguments
    pub command_arguments: Vec<String>,
    /// Output format
    pub output_format: OutputFormat,
}

/// CLI command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLICommand {
    /// Command name
    pub name: String,
    /// Command description
    pub description: String,
    /// Command category
    pub category: CommandCategory,
    /// Required arguments
    pub required_arguments: Vec<CLIArgument>,
    /// Optional arguments
    pub optional_arguments: Vec<CLIArgument>,
    /// Command handler
    pub handler: String,
    /// Command aliases
    pub aliases: Vec<String>,
    /// Command examples
    pub examples: Vec<String>,
    /// Command help text
    pub help_text: String,
}

/// CLI argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    pub description: String,
    /// Argument type
    pub argument_type: ArgumentType,
    /// Is required
    pub required: bool,
    /// Default value
    pub default_value: Option<String>,
    /// Validation rules
    pub validation_rules: Vec<String>,
}

/// Argument type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum ArgumentType {
    /// String argument
    String,
    /// Integer argument
    Integer,
    /// Float argument
    Float,
    /// Boolean argument
    Boolean,
    /// File path argument
    FilePath,
    /// Directory path argument
    DirectoryPath,
    /// URL argument
    Url,
    /// Email argument
    Email,
    /// IP address argument
    IpAddress,
    /// Port number argument
    Port,
    /// Hash argument
    Hash,
    /// Address argument
    Address,
}

/// Command category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandCategory {
    /// Node management commands
    Node,
    /// Wallet commands
    Wallet,
    /// Blockchain commands
    Blockchain,
    /// Network commands
    Network,
    /// Administration commands
    Admin,
    /// System commands
    System,
    /// Help commands
    Help,
}

/// CLI statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIStats {
    /// Total commands executed
    pub total_commands_executed: u64,
    /// Successful commands
    pub successful_commands: u64,
    /// Failed commands
    pub failed_commands: u64,
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Most used commands
    pub most_used_commands: HashMap<String, u64>,
    /// Command success rate
    pub command_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last command timestamp
    pub last_command_timestamp: Option<u64>,
    /// Interactive mode usage
    pub interactive_mode_usage: u64,
    /// Batch mode usage
    pub batch_mode_usage: u64,
}

impl Default for CLIStats {
    fn default() -> Self {
        Self {
            total_commands_executed: 0,
            successful_commands: 0,
            failed_commands: 0,
            average_execution_time_ms: 0.0,
            most_used_commands: HashMap::new(),
            command_success_rate: 0.0,
            uptime_seconds: 0,
            last_command_timestamp: None,
            interactive_mode_usage: 0,
            batch_mode_usage: 0,
        }
    }
}

/// CLI session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLISession {
    /// Session ID
    pub session_id: String,
    /// Session start time
    pub session_start_time: u64,
    /// Commands executed in session
    pub commands_executed: u64,
    /// Session duration in seconds
    pub session_duration_seconds: u64,
    /// Current working directory
    pub current_working_directory: String,
    /// Environment variables
    pub environment_variables: HashMap<String, String>,
    /// Session configuration
    pub session_config: CLIConfig,
}

/// CLI context for command execution
#[derive(Clone)]
pub struct CLIContext {
    /// Current session
    pub session: CLISession,
    /// Node reference
    pub node: Option<Arc<RwLock<crate::node::IppanNode>>>,
    /// Configuration
    pub config: CLIConfig,
    /// Statistics
    pub stats: Arc<RwLock<CLIStats>>,
    /// Command history
    pub command_history: Arc<RwLock<Vec<String>>>,
    /// Environment variables
    pub environment: HashMap<String, String>,
}

impl CLIContext {
    /// Create a new CLI context
    pub fn new(config: CLIConfig) -> Self {
        let session = CLISession {
            session_id: uuid::Uuid::new_v4().to_string(),
            session_start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            commands_executed: 0,
            session_duration_seconds: 0,
            current_working_directory: std::env::current_dir().unwrap_or_default().to_string_lossy().to_string(),
            environment_variables: std::env::vars().collect(),
            session_config: config.clone(),
        };
        
        Self {
            session,
            node: None,
            config,
            stats: Arc::new(RwLock::new(CLIStats::default())),
            command_history: Arc::new(RwLock::new(Vec::new())),
            environment: std::env::vars().collect(),
        }
    }
    
    /// Update session duration
    pub fn update_session_duration(&mut self) {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.session.session_duration_seconds = current_time - self.session.session_start_time;
    }
    
    /// Add command to history
    pub async fn add_command_to_history(&self, command: &str) {
        let mut history = self.command_history.write().await;
        history.push(command.to_string());
        
        // Trim history if it exceeds the limit
        if history.len() > self.config.command_history_size {
            history.remove(0);
        }
    }
    
    /// Get command history
    pub async fn get_command_history(&self) -> Vec<String> {
        let history = self.command_history.read().await;
        history.clone()
    }
    
    /// Update statistics
    pub async fn update_stats(&self, command_name: &str, execution_time_ms: u64, success: bool) {
        let mut stats = self.stats.write().await;
        
        stats.total_commands_executed += 1;
        if success {
            stats.successful_commands += 1;
        } else {
            stats.failed_commands += 1;
        }
        
        // Update average execution time
        let total = stats.total_commands_executed as f64;
        stats.average_execution_time_ms = 
            (stats.average_execution_time_ms * (total - 1.0) + execution_time_ms as f64) / total;
        
        // Update most used commands
        *stats.most_used_commands.entry(command_name.to_string()).or_insert(0) += 1;
        
        // Update success rate
        stats.command_success_rate = stats.successful_commands as f64 / total;
        
        // Update timestamps
        stats.last_command_timestamp = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - self.session.session_start_time;
    }
    
    /// Get statistics
    pub async fn get_stats(&self) -> CLIStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}
