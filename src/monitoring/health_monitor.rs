use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use crate::node::{IppanNode, NodeStatus};

pub struct HealthMonitor {
    node: Arc<RwLock<IppanNode>>,
    check_interval: Duration,
    recovery_attempts: u32,
    max_recovery_attempts: u32,
}

impl HealthMonitor {
    pub fn new(node: Arc<RwLock<IppanNode>>, check_interval: Duration) -> Self {
        HealthMonitor {
            node,
            check_interval,
            recovery_attempts: 0,
            max_recovery_attempts: 5,
        }
    }

    /// Start the health monitoring loop
    pub async fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.perform_health_check().await {
                log::error!("Health check failed: {}", e);
            }
        }
    }

    /// Perform a health check and trigger recovery if needed
    async fn perform_health_check(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let health = {
            let node_guard = self.node.read().await;
            node_guard.health_check().await
        };

        log::debug!("Health check - Status: {:?}, Uptime: {:?}", 
                   health.status, health.uptime);

        match health.status {
            NodeStatus::Unhealthy => {
                log::warn!("Node is unhealthy, attempting recovery");
                self.attempt_recovery().await?;
            }
            NodeStatus::Degraded => {
                log::info!("Node is degraded, attempting graceful recovery");
                self.attempt_graceful_recovery().await?;
            }
            NodeStatus::Healthy => {
                // Reset recovery attempts counter on healthy status
                self.recovery_attempts = 0;
                log::debug!("Node is healthy");
            }
            _ => {
                log::debug!("Node status: {:?}", health.status);
            }
        }

        Ok(())
    }

    /// Attempt emergency recovery
    async fn attempt_recovery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.recovery_attempts >= self.max_recovery_attempts {
            log::error!("Max recovery attempts reached, node may need manual intervention");
            return Ok(());
        }

        self.recovery_attempts += 1;
        log::warn!("Attempting recovery (attempt {}/{})", 
                   self.recovery_attempts, self.max_recovery_attempts);

        let mut node_guard = self.node.write().await;
        let recovery_successful = node_guard.attempt_recovery().await;

        if recovery_successful {
            log::info!("Recovery successful");
            self.recovery_attempts = 0;
        } else {
            log::error!("Recovery failed");
        }

        Ok(())
    }

    /// Attempt graceful recovery
    async fn attempt_graceful_recovery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Attempting graceful recovery");

        let mut node_guard = self.node.write().await;
        let recovery_successful = node_guard.attempt_recovery().await;

        if recovery_successful {
            log::info!("Graceful recovery successful");
        } else {
            log::warn!("Graceful recovery failed, will retry on next health check");
        }

        Ok(())
    }

    /// Get current recovery attempts count
    pub fn get_recovery_attempts(&self) -> u32 {
        self.recovery_attempts
    }

    /// Reset recovery attempts counter
    pub fn reset_recovery_attempts(&mut self) {
        self.recovery_attempts = 0;
    }

    /// Set maximum recovery attempts
    pub fn set_max_recovery_attempts(&mut self, max_attempts: u32) {
        self.max_recovery_attempts = max_attempts;
    }
}

/// Start the health monitoring service
pub async fn start_health_monitor(node: Arc<RwLock<IppanNode>>) {
    let check_interval = Duration::from_secs(30); // Check every 30 seconds
    let mut monitor = HealthMonitor::new(node, check_interval);
    
    log::info!("Starting health monitor with {} second intervals", check_interval.as_secs());
    
    if let Err(e) = monitor.start_monitoring().await {
        log::error!("Health monitor failed: {}", e);
    }
} 