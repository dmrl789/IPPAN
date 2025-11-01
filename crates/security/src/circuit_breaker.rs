use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for the circuit breaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before the circuit is opened.
    pub failure_threshold: u32,
    /// Number of successful calls required in half-open state to close the circuit.
    pub half_open_success_threshold: u32,
    /// Cooldown period in seconds before retrying after the circuit is opened.
    pub recovery_timeout_secs: u64,
}

impl CircuitBreakerConfig {
    fn recovery_timeout(&self) -> Duration {
        Duration::from_secs(self.recovery_timeout_secs)
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            half_open_success_threshold: 3,
            recovery_timeout_secs: 30,
        }
    }
}

/// Runtime state for the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CircuitBreakerState {
    /// Normal operation.
    Closed,
    /// Temporarily blocking requests because too many failures occurred.
    Open,
    /// Allowing a limited number of requests to determine if recovery happened.
    HalfOpen,
}

#[derive(Debug)]
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

/// Simple asynchronous circuit breaker implementation used by the security layer.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Arc<RwLock<CircuitBreakerInner>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker using the provided configuration.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: Arc::new(RwLock::new(CircuitBreakerInner::default())),
        }
    }

    /// Determine whether a request is allowed to proceed based on the current state.
    pub async fn can_execute(&self) -> bool {
        let mut inner = self.inner.write().await;

        match inner.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::HalfOpen => {
                // Allow the request but keep track of how many succeed before closing the circuit.
                if inner.success_count >= self.config.half_open_success_threshold {
                    inner.state = CircuitBreakerState::Closed;
                    inner.failure_count = 0;
                    inner.success_count = 0;
                    inner.last_failure = None;
                }
                true
            }
            CircuitBreakerState::Open => {
                // Give the system a chance to recover after the timeout has elapsed.
                if let Some(last_failure) = inner.last_failure {
                    if last_failure.elapsed() >= self.config.recovery_timeout() {
                        inner.state = CircuitBreakerState::HalfOpen;
                        inner.success_count = 0;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Record a successful execution. Used to move from HalfOpen back to Closed.
    pub async fn record_success(&self) {
        let mut inner = self.inner.write().await;

        match inner.state {
            CircuitBreakerState::Closed => {
                inner.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= self.config.half_open_success_threshold {
                    inner.state = CircuitBreakerState::Closed;
                    inner.failure_count = 0;
                    inner.success_count = 0;
                    inner.last_failure = None;
                }
            }
            CircuitBreakerState::Open => {
                // Ignore successes while the breaker is open. They shouldn't happen because execution is blocked.
            }
        }
    }

    /// Record a failed execution and update the breaker state accordingly.
    pub async fn record_failure(&self) {
        let mut inner = self.inner.write().await;
        inner.failure_count = inner.failure_count.saturating_add(1);

        match inner.state {
            CircuitBreakerState::Closed => {
                if inner.failure_count >= self.config.failure_threshold {
                    inner.state = CircuitBreakerState::Open;
                    inner.success_count = 0;
                    inner.last_failure = Some(Instant::now());
                }
            }
            CircuitBreakerState::HalfOpen | CircuitBreakerState::Open => {
                inner.state = CircuitBreakerState::Open;
                inner.success_count = 0;
                inner.last_failure = Some(Instant::now());
            }
        }
    }

    /// Manually reset the circuit breaker to the Closed state.
    pub async fn reset(&self) {
        let mut inner = self.inner.write().await;
        *inner = CircuitBreakerInner::default();
    }

    /// Obtain the current state without requiring an async context.
    pub fn get_state(&self) -> CircuitBreakerState {
        self.inner.blocking_read().state
    }
}
