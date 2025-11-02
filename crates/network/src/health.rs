use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Health status for a peer connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerHealth {
    /// Connection is healthy
    Healthy,
    /// Connection is degraded but still functional
    Degraded,
    /// Connection is unhealthy and should be replaced
    Unhealthy,
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Interval between health checks
    pub check_interval: Duration,
    /// Timeout for health check requests
    pub check_timeout: Duration,
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
    /// Time without activity before marking as stale
    pub stale_threshold: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(5),
            failure_threshold: 3,
            stale_threshold: Duration::from_secs(300),
        }
    }
}

/// Health tracking for a single peer
#[derive(Debug, Clone)]
struct PeerHealthStatus {
    health: PeerHealth,
    consecutive_failures: u32,
    last_success: Instant,
    last_check: Instant,
    total_checks: u64,
    successful_checks: u64,
}

impl PeerHealthStatus {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            health: PeerHealth::Healthy,
            consecutive_failures: 0,
            last_success: now,
            last_check: now,
            total_checks: 0,
            successful_checks: 0,
        }
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_success = Instant::now();
        self.last_check = Instant::now();
        self.total_checks += 1;
        self.successful_checks += 1;
        self.health = PeerHealth::Healthy;
    }

    fn record_failure(&mut self, failure_threshold: u32) {
        self.consecutive_failures += 1;
        self.last_check = Instant::now();
        self.total_checks += 1;

        if self.consecutive_failures >= failure_threshold {
            self.health = PeerHealth::Unhealthy;
        } else if self.consecutive_failures >= failure_threshold / 2 {
            self.health = PeerHealth::Degraded;
        }
    }

    fn is_stale(&self, stale_threshold: Duration) -> bool {
        self.last_success.elapsed() >= stale_threshold
    }

    fn success_rate(&self) -> f64 {
        if self.total_checks == 0 {
            return 1.0;
        }
        self.successful_checks as f64 / self.total_checks as f64
    }
}

/// Connection health monitor
#[derive(Debug)]
pub struct HealthMonitor {
    config: HealthCheckConfig,
    peer_health: RwLock<HashMap<String, PeerHealthStatus>>,
    last_cleanup: RwLock<Instant>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            peer_health: RwLock::new(HashMap::new()),
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Record a successful health check for a peer
    pub fn record_success(&self, peer_address: &str) {
        let mut health = self.peer_health.write();
        let status = health
            .entry(peer_address.to_string())
            .or_insert_with(PeerHealthStatus::new);
        status.record_success();
        debug!("Health check succeeded for peer {}", peer_address);
    }

    /// Record a failed health check for a peer
    pub fn record_failure(&self, peer_address: &str) {
        let mut health = self.peer_health.write();
        let status = health
            .entry(peer_address.to_string())
            .or_insert_with(PeerHealthStatus::new);
        status.record_failure(self.config.failure_threshold);

        match status.health {
            PeerHealth::Unhealthy => {
                warn!(
                    "Peer {} is unhealthy after {} consecutive failures",
                    peer_address, status.consecutive_failures
                );
            }
            PeerHealth::Degraded => {
                warn!("Peer {} connection is degraded", peer_address);
            }
            PeerHealth::Healthy => {}
        }
    }

    /// Get the health status of a peer
    pub fn get_health(&self, peer_address: &str) -> PeerHealth {
        let health = self.peer_health.read();
        health
            .get(peer_address)
            .map(|status| {
                // Check for stale connections
                if status.is_stale(self.config.stale_threshold) {
                    PeerHealth::Unhealthy
                } else {
                    status.health
                }
            })
            .unwrap_or(PeerHealth::Healthy)
    }

    /// Check if a peer is healthy
    pub fn is_healthy(&self, peer_address: &str) -> bool {
        matches!(self.get_health(peer_address), PeerHealth::Healthy)
    }

    /// Get detailed health statistics for a peer
    pub fn get_stats(&self, peer_address: &str) -> Option<PeerHealthStats> {
        let health = self.peer_health.read();
        health.get(peer_address).map(|status| PeerHealthStats {
            health: status.health,
            consecutive_failures: status.consecutive_failures,
            success_rate: status.success_rate(),
            total_checks: status.total_checks,
            last_success_seconds: status.last_success.elapsed().as_secs(),
            last_check_seconds: status.last_check.elapsed().as_secs(),
        })
    }

    /// Get list of unhealthy peers
    pub fn unhealthy_peers(&self) -> Vec<String> {
        let health = self.peer_health.read();
        health
            .iter()
            .filter(|(_, status)| {
                status.health == PeerHealth::Unhealthy
                    || status.is_stale(self.config.stale_threshold)
            })
            .map(|(addr, _)| addr.clone())
            .collect()
    }

    /// Remove a peer from health monitoring
    pub fn remove_peer(&self, peer_address: &str) {
        self.peer_health.write().remove(peer_address);
        debug!("Removed peer {} from health monitoring", peer_address);
    }

    /// Perform periodic cleanup of stale entries
    pub fn cleanup(&self) {
        let mut last_cleanup = self.last_cleanup.write();
        if last_cleanup.elapsed() >= Duration::from_secs(600) {
            let mut health = self.peer_health.write();
            health.retain(|addr, status| {
                let should_keep = !status.is_stale(self.config.stale_threshold * 2);
                if !should_keep {
                    debug!("Removing stale health entry for peer {}", addr);
                }
                should_keep
            });
            *last_cleanup = Instant::now();
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(HealthCheckConfig::default())
    }
}

/// Health statistics for a peer
#[derive(Debug, Clone)]
pub struct PeerHealthStats {
    pub health: PeerHealth,
    pub consecutive_failures: u32,
    pub success_rate: f64,
    pub total_checks: u64,
    pub last_success_seconds: u64,
    pub last_check_seconds: u64,
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_health_tracking() {
        let monitor = HealthMonitor::default();
        let peer = "127.0.0.1:9000";

        // Initial state should be healthy
        assert!(monitor.is_healthy(peer));

        // Record success
        monitor.record_success(peer);
        assert_eq!(monitor.get_health(peer), PeerHealth::Healthy);

        // Record some failures
        monitor.record_failure(peer);
        monitor.record_failure(peer);

        // Should be degraded after 2 failures (threshold is 3)
        assert_eq!(monitor.get_health(peer), PeerHealth::Degraded);

        // One more failure should make it unhealthy
        monitor.record_failure(peer);
        assert_eq!(monitor.get_health(peer), PeerHealth::Unhealthy);
    }

    #[test]
    fn test_health_stats() {
        let monitor = HealthMonitor::default();
        let peer = "127.0.0.1:9000";

        monitor.record_success(peer);
        monitor.record_success(peer);
        monitor.record_failure(peer);

        let stats = monitor.get_stats(peer).unwrap();
        assert_eq!(stats.total_checks, 3);
        assert!((stats.success_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_unhealthy_peers() {
        let monitor = HealthMonitor::default();

        let peer1 = "127.0.0.1:9000";
        let peer2 = "127.0.0.1:9001";

        monitor.record_success(peer1);

        // Make peer2 unhealthy
        for _ in 0..3 {
            monitor.record_failure(peer2);
        }

        let unhealthy = monitor.unhealthy_peers();
        assert_eq!(unhealthy.len(), 1);
        assert!(unhealthy.contains(&peer2.to_string()));
    }
}
