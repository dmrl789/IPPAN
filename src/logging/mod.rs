//! Comprehensive logging and monitoring system for IPPAN
//! 
//! Provides structured logging, error tracking, performance monitoring, and alerting

pub mod structured_logger;
pub mod log_aggregator;
pub mod log_analyzer;
pub mod log_retention;
pub mod log_export;
pub mod log_dashboard;

use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main logging system that orchestrates all logging components
pub struct LoggingSystem {
    structured_logger: Arc<structured_logger::StructuredLogger>,
    log_aggregator: Arc<log_aggregator::LogAggregator>,
    log_analyzer: Arc<log_analyzer::LogAnalyzer>,
    log_retention: Arc<log_retention::LogRetentionManager>,
    log_export: Arc<log_export::LogExportManager>,
    log_dashboard: Arc<log_dashboard::LogDashboard>,
}

impl LoggingSystem {
    /// Create a new logging system
    pub fn new() -> Self {
        let structured_logger = Arc::new(structured_logger::StructuredLogger::new());
        let log_aggregator = Arc::new(log_aggregator::LogAggregator::new(Arc::clone(&structured_logger)));
        let log_analyzer = Arc::new(log_analyzer::LogAnalyzer::new(Arc::clone(&structured_logger)));
        let log_retention = Arc::new(log_retention::LogRetentionManager::new(Arc::clone(&structured_logger)));
        let log_export = Arc::new(log_export::LogExportManager::new(Arc::clone(&structured_logger)));
        let log_dashboard = Arc::new(log_dashboard::LogDashboard::new(Arc::clone(&structured_logger)));

        Self {
            structured_logger,
            log_aggregator,
            log_analyzer,
            log_retention,
            log_export,
            log_dashboard,
        }
    }

    /// Start the logging system
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting IPPAN logging system...");
        
        // Initialize structured logger
        self.structured_logger.start().await?;
        
        // Start log aggregator
        self.log_aggregator.start().await?;
        
        // Start log analyzer
        self.log_analyzer.start().await?;
        
        // Start log retention manager
        self.log_retention.start().await?;
        
        // Start log export manager
        self.log_export.start().await?;
        
        // Start log dashboard
        self.log_dashboard.start().await?;
        
        log::info!("Logging system started successfully");
        Ok(())
    }

    /// Stop the logging system
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping IPPAN logging system...");
        
        self.structured_logger.stop().await?;
        self.log_aggregator.stop().await?;
        self.log_analyzer.stop().await?;
        self.log_retention.stop().await?;
        self.log_export.stop().await?;
        self.log_dashboard.stop().await?;
        
        log::info!("Logging system stopped");
        Ok(())
    }

    /// Get the structured logger
    pub fn get_structured_logger(&self) -> Arc<structured_logger::StructuredLogger> {
        Arc::clone(&self.structured_logger)
    }

    /// Get the log aggregator
    pub fn get_log_aggregator(&self) -> Arc<log_aggregator::LogAggregator> {
        Arc::clone(&self.log_aggregator)
    }

    /// Get the log analyzer
    pub fn get_log_analyzer(&self) -> Arc<log_analyzer::LogAnalyzer> {
        Arc::clone(&self.log_analyzer)
    }

    /// Get the log retention manager
    pub fn get_log_retention(&self) -> Arc<log_retention::LogRetentionManager> {
        Arc::clone(&self.log_retention)
    }

    /// Get the log export manager
    pub fn get_log_export(&self) -> Arc<log_export::LogExportManager> {
        Arc::clone(&self.log_export)
    }

    /// Get the log dashboard
    pub fn get_log_dashboard(&self) -> Arc<log_dashboard::LogDashboard> {
        Arc::clone(&self.log_dashboard)
    }

    /// Get logging statistics
    pub async fn get_logging_statistics(&self) -> LoggingStatistics {
        let structured_stats = self.structured_logger.get_statistics().await;
        let aggregator_stats = self.log_aggregator.get_statistics().await;
        let analyzer_stats = self.log_analyzer.get_statistics().await;
        let retention_stats = self.log_retention.get_statistics().await;
        let export_stats = self.log_export.get_statistics().await;

        LoggingStatistics {
            structured_logger: structured_stats,
            log_aggregator: aggregator_stats,
            log_analyzer: analyzer_stats,
            log_retention: retention_stats,
            log_export: export_stats,
        }
    }
}

/// Comprehensive logging statistics
#[derive(Debug, Clone)]
pub struct LoggingStatistics {
    pub structured_logger: structured_logger::LoggerStatistics,
    pub log_aggregator: log_aggregator::AggregatorStatistics,
    pub log_analyzer: log_analyzer::AnalyzerStatistics,
    pub log_retention: log_retention::RetentionStatistics,
    pub log_export: log_export::ExportStatistics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logging_system_creation() {
        let logging_system = LoggingSystem::new();
        let stats = logging_system.get_logging_statistics().await;
        
        // Verify all components are initialized
        assert_eq!(stats.structured_logger.total_logs, 0);
        assert_eq!(stats.log_aggregator.total_aggregated_logs, 0);
        assert_eq!(stats.log_analyzer.total_analyses_performed, 0);
        assert_eq!(stats.log_retention.total_cleanup_operations, 0);
        assert_eq!(stats.log_export.total_export_jobs, 0);
    }

    #[tokio::test]
    async fn test_logging_system_start_stop() {
        let logging_system = LoggingSystem::new();
        
        // Start the system
        logging_system.start().await.unwrap();
        
        // Stop the system
        logging_system.stop().await.unwrap();
    }
}
