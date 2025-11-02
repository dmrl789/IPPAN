use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// State of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Normal operation.
    Closed,
    /// Requests are blocked until the recovery timeout elapses.
    Open,
    /// Allow a limited number of test requests to determine recovery.
    HalfOpen,
}

/// Configuration for the circuit breaker implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Consecutive failures required before opening the circuit.
    pub failure_threshold: u32,
    /// Seconds to wait before transitioning from `Open` to `HalfOpen`.
    pub recovery_timeout_secs: u64,
    /// Successful calls required in `HalfOpen` state to fully close circuit.
    pub half_open_success_threshold: u32,
    /// Failures tolerated in `HalfOpen` state before re-opening circuit.
    pub half_open_failure_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout_secs: 30,
            half_open_success_threshold: 3,
            half_open_failure_threshold: 1,
        }
    }
}

struct CircuitBreakerInner {
    state: CircuitBreakerState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
}

impl Default for CircuitBreakerInner {
    fn default() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
        }
    }
}

/// Tokio-friendly circuit breaker primitive.
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Mutex<CircuitBreakerInner>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker instance.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(CircuitBreakerInner::default()),
        }
    }

    /// Determine whether execution should proceed.
    pub async fn can_execute(&self) -> bool {
        let mut inner = self.inner.lock();
        match inner.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::HalfOpen => {
                debug!("circuit breaker half-open: allowing probe request");
                true
            }
            CircuitBreakerState::Open => {
                if let Some(last_failure) = inner.last_failure {
                    if last_failure.elapsed() >= self.recovery_timeout() {
                        info!("circuit breaker transitioning to half-open state");
                        inner.state = CircuitBreakerState::HalfOpen;
                        inner.failure_count = 0;
                        inner.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful execution.
    pub async fn record_success(&self) {
        let mut inner = self.inner.lock();
        match inner.state {
            CircuitBreakerState::Closed => {
                inner.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= self.config.half_open_success_threshold {
                    info!("circuit breaker closing after successful half-open probes");
                    inner.state = CircuitBreakerState::Closed;
                    inner.success_count = 0;
                    inner.failure_count = 0;
                    inner.last_failure = None;
                }
            }
            CircuitBreakerState::Open => {
                // Ignore successes while open; transition happens via can_execute.
            }
        }
    }

    /// Record a failed execution.
    pub async fn record_failure(&self) {
        let mut inner = self.inner.lock();
        inner.failure_count = inner.failure_count.saturating_add(1);
        inner.last_failure = Some(Instant::now());

        match inner.state {
            CircuitBreakerState::Closed => {
                if inner.failure_count >= self.config.failure_threshold {
                    warn!(
                        "circuit breaker opening after {count} failures",
                        count = inner.failure_count
                    );
                    inner.state = CircuitBreakerState::Open;
                    inner.success_count = 0;
                }
            }
            CircuitBreakerState::HalfOpen => {
                inner.success_count = 0;
                if inner.failure_count >= self.config.half_open_failure_threshold {
                    warn!("circuit breaker returning to open state from half-open");
                    inner.state = CircuitBreakerState::Open;
                }
            }
            CircuitBreakerState::Open => {
                // Already open; keep the last failure timestamp fresh to extend timeout.
            }
        }
    }

    /// Reset the circuit breaker to the closed state.
    pub async fn reset(&self) {
        let mut inner = self.inner.lock();
        *inner = CircuitBreakerInner::default();
    }

    /// Retrieve the current circuit breaker state.
    pub fn get_state(&self) -> CircuitBreakerState {
        self.inner.lock().state
    }

    fn recovery_timeout(&self) -> Duration {
        Duration::from_secs(self.config.recovery_timeout_secs)
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn circuit_opens_after_failures() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout_secs: 60,
            half_open_success_threshold: 1,
            half_open_failure_threshold: 1,
        });

        breaker.record_failure().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);

        breaker.record_failure().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn half_open_after_timeout() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 0,
            half_open_success_threshold: 1,
            half_open_failure_threshold: 1,
        });

        breaker.record_failure().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);

        // With zero timeout we should transition immediately.
        assert!(breaker.can_execute().await);
        assert_eq!(breaker.get_state(), CircuitBreakerState::HalfOpen);
    }

    #[tokio::test]
    async fn closes_after_half_open_successes() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 0,
            half_open_success_threshold: 2,
            half_open_failure_threshold: 1,
        });

        breaker.record_failure().await;
        assert!(breaker.can_execute().await);
        assert_eq!(breaker.get_state(), CircuitBreakerState::HalfOpen);

        breaker.record_success().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::HalfOpen);

        breaker.record_success().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn returns_to_open_on_half_open_failure() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 0,
            half_open_success_threshold: 2,
            half_open_failure_threshold: 1,
        });

        breaker.record_failure().await;
        assert!(breaker.can_execute().await);
        assert_eq!(breaker.get_state(), CircuitBreakerState::HalfOpen);

        breaker.record_failure().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn open_state_blocks_until_timeout() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 1,
            half_open_success_threshold: 1,
            half_open_failure_threshold: 1,
        });

        breaker.record_failure().await;
        assert!(!breaker.can_execute().await);
        sleep(Duration::from_secs(1)).await;
        assert!(breaker.can_execute().await);
    }
}
