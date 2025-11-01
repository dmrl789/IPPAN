use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Circuit breaker states
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Configuration for the circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before the breaker opens
    pub failure_threshold: u32,
    /// Number of consecutive successes required to close the breaker from half-open
    pub success_threshold: u32,
    /// Amount of time (in milliseconds) to wait before attempting recovery from open state
    pub recovery_timeout_ms: u64,
}

impl CircuitBreakerConfig {
    pub fn recovery_timeout(&self) -> Duration {
        Duration::from_millis(self.recovery_timeout_ms.max(1))
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            recovery_timeout_ms: 30_000,
        }
    }
}

/// Asynchronous-friendly circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitBreakerState>,
    failure_count: RwLock<u32>,
    success_count: RwLock<u32>,
    last_transition: RwLock<Option<Instant>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitBreakerState::Closed),
            failure_count: RwLock::new(0),
            success_count: RwLock::new(0),
            last_transition: RwLock::new(None),
        }
    }

    /// Determine if the protected operation can execute
    pub async fn can_execute(&self) -> bool {
        self.can_execute_inner()
    }

    fn can_execute_inner(&self) -> bool {
        match *self.state.read() {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::HalfOpen => true,
            CircuitBreakerState::Open => {
                let should_attempt_recovery = {
                    let guard = self.last_transition.read();
                    match *guard {
                        Some(instant) => instant.elapsed() >= self.config.recovery_timeout(),
                        None => true,
                    }
                };

                if should_attempt_recovery {
                    *self.state.write() = CircuitBreakerState::HalfOpen;
                    *self.success_count.write() = 0;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful attempt
    pub async fn record_success(&self) {
        self.record_success_inner();
    }

    fn record_success_inner(&self) {
        match *self.state.read() {
            CircuitBreakerState::Closed => {
                *self.failure_count.write() = 0;
            }
            CircuitBreakerState::HalfOpen => {
                let mut successes = self.success_count.write();
                *successes += 1;

                if *successes >= self.config.success_threshold.max(1) {
                    *self.state.write() = CircuitBreakerState::Closed;
                    *self.failure_count.write() = 0;
                    *successes = 0;
                    *self.last_transition.write() = Some(Instant::now());
                }
            }
            CircuitBreakerState::Open => {
                // Success from open state should not happen, but if it does treat it as recovery attempt.
                *self.state.write() = CircuitBreakerState::HalfOpen;
                *self.success_count.write() = 1;
                *self.last_transition.write() = Some(Instant::now());
            }
        }
    }

    /// Record a failed attempt
    pub async fn record_failure(&self) {
        self.record_failure_inner();
    }

    fn record_failure_inner(&self) {
        let current_state = *self.state.read();
        match current_state {
            CircuitBreakerState::Closed => {
                let mut failures = self.failure_count.write();
                *failures += 1;
                if *failures >= self.config.failure_threshold.max(1) {
                    *failures = 0;
                    *self.state.write() = CircuitBreakerState::Open;
                    *self.last_transition.write() = Some(Instant::now());
                }
                *self.success_count.write() = 0;
            }
            CircuitBreakerState::HalfOpen => {
                *self.state.write() = CircuitBreakerState::Open;
                *self.failure_count.write() = 0;
                *self.success_count.write() = 0;
                *self.last_transition.write() = Some(Instant::now());
            }
            CircuitBreakerState::Open => {
                *self.last_transition.write() = Some(Instant::now());
            }
        }
    }

    /// Current breaker state snapshot
    pub fn get_state(&self) -> CircuitBreakerState {
        *self.state.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn transitions_to_open_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            recovery_timeout_ms: 50,
        };

        let breaker = CircuitBreaker::new(config);

        assert!(breaker.can_execute().await);
        breaker.record_failure().await;
        assert!(breaker.can_execute().await);
        breaker.record_failure().await;
        assert!(!breaker.can_execute().await);
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn recovers_after_timeout_and_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            recovery_timeout_ms: 25,
        };

        let breaker = CircuitBreaker::new(config);

        breaker.record_failure().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);
        assert!(!breaker.can_execute().await);

        thread::sleep(Duration::from_millis(30));
        assert!(breaker.can_execute().await);
        assert_eq!(breaker.get_state(), CircuitBreakerState::HalfOpen);

        breaker.record_success().await;
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);
    }
}
