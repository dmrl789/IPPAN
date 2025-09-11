//! Interactive shell for IPPAN CLI
//! 
//! Implements an interactive shell with command completion, history,
//! and user-friendly interface.

use crate::{Result, IppanError, TransactionHash};
use super::{CLIContext, CLIResult, OutputFormat, CLIManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Interactive shell configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveShellConfig {
    /// Enable colored output
    pub enable_colored_output: bool,
    /// Enable command completion
    pub enable_command_completion: bool,
    /// Enable command history
    pub enable_command_history: bool,
    /// Command history size
    pub command_history_size: usize,
    /// Prompt string
    pub prompt_string: String,
    /// Welcome message
    pub welcome_message: String,
    /// Goodbye message
    pub goodbye_message: String,
    /// Enable auto-save
    pub enable_auto_save: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval_seconds: u64,
    /// Enable command aliases
    pub enable_command_aliases: bool,
    /// Custom command aliases
    pub command_aliases: HashMap<String, String>,
}

impl Default for InteractiveShellConfig {
    fn default() -> Self {
        let mut command_aliases = HashMap::new();
        command_aliases.insert("ls".to_string(), "list".to_string());
        command_aliases.insert("ps".to_string(), "status".to_string());
        command_aliases.insert("q".to_string(), "quit".to_string());
        command_aliases.insert("h".to_string(), "help".to_string());
        command_aliases.insert("exit".to_string(), "quit".to_string());
        
        Self {
            enable_colored_output: true,
            enable_command_completion: true,
            enable_command_history: true,
            command_history_size: 1000,
            prompt_string: "ippan> ".to_string(),
            welcome_message: "Welcome to IPPAN Interactive Shell!".to_string(),
            goodbye_message: "Goodbye!".to_string(),
            enable_auto_save: true,
            auto_save_interval_seconds: 300, // 5 minutes
            enable_command_aliases: true,
            command_aliases,
        }
    }
}

/// Interactive shell
pub struct InteractiveShell {
    /// Configuration
    config: InteractiveShellConfig,
    /// CLI manager
    cli_manager: Arc<CLIManager>,
    /// Command history
    command_history: Arc<RwLock<Vec<String>>>,
    /// Current command index
    current_command_index: Arc<RwLock<usize>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

/// Interactive shell statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveShellStats {
    /// Total commands executed
    pub total_commands_executed: u64,
    /// Successful commands
    pub successful_commands: u64,
    /// Failed commands
    pub failed_commands: u64,
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Session duration in seconds
    pub session_duration_seconds: u64,
    /// Commands per minute
    pub commands_per_minute: f64,
    /// Most used commands
    pub most_used_commands: HashMap<String, u64>,
    /// Command success rate
    pub command_success_rate: f64,
    /// Last command timestamp
    pub last_command_timestamp: Option<u64>,
}

impl Default for InteractiveShellStats {
    fn default() -> Self {
        Self {
            total_commands_executed: 0,
            successful_commands: 0,
            failed_commands: 0,
            average_execution_time_ms: 0.0,
            session_duration_seconds: 0,
            commands_per_minute: 0.0,
            most_used_commands: HashMap::new(),
            command_success_rate: 0.0,
            last_command_timestamp: None,
        }
    }
}

impl InteractiveShell {
    /// Create a new interactive shell
    pub fn new(config: InteractiveShellConfig, cli_manager: Arc<CLIManager>) -> Self {
        Self {
            config,
            cli_manager,
            command_history: Arc::new(RwLock::new(Vec::new())),
            current_command_index: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the interactive shell
    pub async fn start(&self) -> Result<()> {
        info!("Starting interactive shell");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Display welcome message
        self.display_welcome_message().await;
        
        // Start auto-save task
        if self.config.enable_auto_save {
            self.start_auto_save_task().await;
        }
        
        info!("Interactive shell started successfully");
        Ok(())
    }
    
    /// Stop the interactive shell
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping interactive shell");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Display goodbye message
        self.display_goodbye_message().await;
        
        info!("Interactive shell stopped");
        Ok(())
    }
    
    /// Run the interactive shell
    pub async fn run(&self) -> Result<()> {
        info!("Running interactive shell");
        
        loop {
            // Check if shell is still running
            {
                let is_running = self.is_running.read().await;
                if !*is_running {
                    break;
                }
            }
            
            // Display prompt
            self.display_prompt().await;
            
            // Read command input
            let command_input = self.read_command_input().await?;
            
            // Process command
            if !command_input.trim().is_empty() {
                self.process_command(&command_input).await?;
            }
        }
        
        info!("Interactive shell finished");
        Ok(())
    }
    
    /// Display welcome message
    async fn display_welcome_message(&self) {
        if self.config.enable_colored_output {
            println!("\x1b[32m{}\x1b[0m", self.config.welcome_message);
        } else {
            println!("{}", self.config.welcome_message);
        }
        
        println!("Type 'help' for available commands or 'quit' to exit.");
        println!();
    }
    
    /// Display goodbye message
    async fn display_goodbye_message(&self) {
        if self.config.enable_colored_output {
            println!("\x1b[32m{}\x1b[0m", self.config.goodbye_message);
        } else {
            println!("{}", self.config.goodbye_message);
        }
    }
    
    /// Display prompt
    async fn display_prompt(&self) {
        if self.config.enable_colored_output {
            print!("\x1b[34m{}\x1b[0m", self.config.prompt_string);
        } else {
            print!("{}", self.config.prompt_string);
        }
        
        // Flush output
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }
    
    /// Read command input
    async fn read_command_input(&self) -> Result<String> {
        use std::io::{self, BufRead};
        
        let stdin = io::stdin();
        let mut line = String::new();
        
        stdin.lock().read_line(&mut line)
            .map_err(|e| IppanError::CLI(format!("Failed to read input: {}", e)))?;
        
        Ok(line.trim().to_string())
    }
    
    /// Process command
    async fn process_command(&self, command_input: &str) -> Result<()> {
        let start_time = Instant::now();
        
        // Handle command aliases
        let processed_command = if self.config.enable_command_aliases {
            self.process_command_aliases(command_input).await
        } else {
            command_input.to_string()
        };
        
        // Execute command
        let result = self.cli_manager.execute_command(&processed_command).await?;
        
        // Display result
        self.display_command_result(&result).await;
        
        // Add to history
        if self.config.enable_command_history {
            self.add_to_command_history(command_input).await;
        }
        
        // Update statistics
        self.update_statistics(&result, start_time.elapsed().as_millis() as u64).await;
        
        Ok(())
    }
    
    /// Process command aliases
    async fn process_command_aliases(&self, command_input: &str) -> String {
        let parts: Vec<&str> = command_input.split_whitespace().collect();
        
        if parts.is_empty() {
            return command_input.to_string();
        }
        
        let first_word = parts[0];
        
        if let Some(alias) = self.config.command_aliases.get(first_word) {
            let mut result = alias.clone();
            if parts.len() > 1 {
                result.push(' ');
                result.push_str(&parts[1..].join(" "));
            }
            result
        } else {
            command_input.to_string()
        }
    }
    
    /// Display command result
    async fn display_command_result(&self, result: &CLIResult) {
        if result.success {
            if let Some(data) = &result.data {
                match result.output_format {
                    OutputFormat::Json => {
                        if self.config.enable_colored_output {
                            println!("\x1b[32m{}\x1b[0m", serde_json::to_string_pretty(data).unwrap_or_default());
                        } else {
                            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
                        }
                    }
                    OutputFormat::Table => {
                        // Simple table display
                        if let Some(array) = data.as_array() {
                            for item in array {
                                println!("{}", serde_json::to_string_pretty(item).unwrap_or_default());
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
                        }
                    }
                    OutputFormat::Plain => {
                        if let Some(string) = data.as_str() {
                            println!("{}", string);
                        } else {
                            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
                        }
                    }
                    _ => {
                        println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
                    }
                }
            }
        } else {
            if let Some(error_message) = &result.error_message {
                if self.config.enable_colored_output {
                    println!("\x1b[31mError: {}\x1b[0m", error_message);
                } else {
                    println!("Error: {}", error_message);
                }
            }
        }
        
        // Display execution time if verbose
        if self.config.enable_colored_output {
            println!("\x1b[90m(executed in {}ms)\x1b[0m", result.execution_time_ms);
        } else {
            println!("(executed in {}ms)", result.execution_time_ms);
        }
    }
    
    /// Add command to history
    async fn add_to_command_history(&self, command: &str) {
        let mut history = self.command_history.write().await;
        history.push(command.to_string());
        
        // Trim history if it exceeds size limit
        if history.len() > self.config.command_history_size {
            history.remove(0);
        }
        
        // Update current command index
        let mut index = self.current_command_index.write().await;
        *index = history.len();
    }
    
    /// Get command history
    pub async fn get_command_history(&self) -> Result<Vec<String>> {
        let history = self.command_history.read().await;
        Ok(history.clone())
    }
    
    /// Clear command history
    pub async fn clear_command_history(&self) -> Result<()> {
        let mut history = self.command_history.write().await;
        history.clear();
        
        let mut index = self.current_command_index.write().await;
        *index = 0;
        
        Ok(())
    }
    
    /// Start auto-save task
    async fn start_auto_save_task(&self) {
        let history = self.command_history.clone();
        let interval = self.config.auto_save_interval_seconds;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(interval));
            
            loop {
                interval_timer.tick().await;
                
                // Save command history to file
                let history_data = {
                    let history = history.read().await;
                    history.clone()
                };
                
                if let Err(e) = tokio::fs::write("cli_history.json", serde_json::to_string_pretty(&history_data).unwrap_or_default()).await {
                    error!("Failed to save command history: {}", e);
                }
            }
        });
    }
    
    /// Update statistics
    async fn update_statistics(&self, result: &CLIResult, execution_time_ms: u64) {
        // This would update shell-specific statistics
        // For now, we'll just log the update
        debug!("Command executed: {} in {}ms", result.command_name, execution_time_ms);
    }
    
    /// Get interactive shell statistics
    pub async fn get_statistics(&self) -> Result<InteractiveShellStats> {
        let history = self.command_history.read().await;
        let session_duration = self.start_time.elapsed().as_secs();
        
        let stats = InteractiveShellStats {
            total_commands_executed: history.len() as u64,
            successful_commands: history.len() as u64, // Simplified
            failed_commands: 0, // Simplified
            average_execution_time_ms: 0.0, // Would be calculated from actual execution times
            session_duration_seconds: session_duration,
            commands_per_minute: if session_duration > 0 {
                (history.len() as f64 * 60.0) / session_duration as f64
            } else {
                0.0
            },
            most_used_commands: HashMap::new(), // Would be calculated from actual command usage
            command_success_rate: 1.0, // Simplified
            last_command_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_interactive_shell_config() {
        let config = InteractiveShellConfig::default();
        assert!(config.enable_colored_output);
        assert!(config.enable_command_completion);
        assert!(config.enable_command_history);
        assert_eq!(config.command_history_size, 1000);
        assert_eq!(config.prompt_string, "ippan> ");
        assert_eq!(config.welcome_message, "Welcome to IPPAN Interactive Shell!");
        assert_eq!(config.goodbye_message, "Goodbye!");
        assert!(config.enable_auto_save);
        assert_eq!(config.auto_save_interval_seconds, 300);
        assert!(config.enable_command_aliases);
        assert!(!config.command_aliases.is_empty());
    }
    
    #[tokio::test]
    async fn test_interactive_shell_stats() {
        let stats = InteractiveShellStats {
            total_commands_executed: 100,
            successful_commands: 95,
            failed_commands: 5,
            average_execution_time_ms: 50.0,
            session_duration_seconds: 3600,
            commands_per_minute: 1.67,
            most_used_commands: HashMap::new(),
            command_success_rate: 0.95,
            last_command_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_commands_executed, 100);
        assert_eq!(stats.successful_commands, 95);
        assert_eq!(stats.failed_commands, 5);
        assert_eq!(stats.average_execution_time_ms, 50.0);
        assert_eq!(stats.session_duration_seconds, 3600);
        assert_eq!(stats.commands_per_minute, 1.67);
        assert_eq!(stats.command_success_rate, 0.95);
        assert!(stats.last_command_timestamp.is_some());
    }
}
