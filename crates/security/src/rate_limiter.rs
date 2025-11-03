use anyhow::Result;
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per second per IP
    pub requests_per_second: u32,
    /// Burst capacity
    pub burst_capacity: u32,
    /// Per-endpoint rate limits
    pub endpoint_limits: HashMap<String, EndpointLimit>,
    /// Global rate limit (requests per second for all IPs combined)
    pub global_requests_per_second: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut endpoint_limits = HashMap::new();

        // Define stricter limits for sensitive endpoints
        endpoint_limits.insert(
            "/tx".to_string(),
            EndpointLimit {
                requests_per_second: 10,
                burst_capacity: 20,
            },
        );

        endpoint_limits.insert(
            "/account".to_string(),
            EndpointLimit {
                requests_per_second: 50,
                burst_capacity: 100,
            },
        );

        endpoint_limits.insert(
            "/block".to_string(),
            EndpointLimit {
                requests_per_second: 100,
                burst_capacity: 200,
            },
        );

        Self {
            requests_per_second: 100,
            burst_capacity: 200,
            endpoint_limits,
            global_requests_per_second: Some(10000),
        }
    }
}

/// Per-endpoint rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointLimit {
    pub requests_per_second: u32,
    pub burst_capacity: u32,
}

/// Rate limiter implementation
pub struct RateLimiter {
    config: RateLimitConfig,
    ip_limiters: Arc<
        RwLock<
            HashMap<
                IpAddr,
                GovernorRateLimiter<
                    governor::state::direct::NotKeyed,
                    governor::state::InMemoryState,
                    governor::clock::DefaultClock,
                >,
            >,
        >,
    >,
    endpoint_limiters: Arc<
        RwLock<
            HashMap<
                String,
                HashMap<
                    IpAddr,
                    GovernorRateLimiter<
                        governor::state::direct::NotKeyed,
                        governor::state::InMemoryState,
                        governor::clock::DefaultClock,
                    >,
                >,
            >,
        >,
    >,
    global_limiter: Option<
        GovernorRateLimiter<
            governor::state::direct::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
    stats: Arc<RwLock<RateLimitStats>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Result<Self> {
        let global_limiter = if let Some(global_rps) = config.global_requests_per_second {
            let quota = Quota::per_second(NonZeroU32::new(global_rps).unwrap());
            Some(GovernorRateLimiter::direct(quota))
        } else {
            None
        };

        Ok(Self {
            config,
            ip_limiters: Arc::new(RwLock::new(HashMap::new())),
            endpoint_limiters: Arc::new(RwLock::new(HashMap::new())),
            global_limiter,
            stats: Arc::new(RwLock::new(RateLimitStats::default())),
        })
    }

    /// Check if a request should be rate limited
    pub async fn check_rate_limit(&self, ip: IpAddr, endpoint: &str) -> Result<bool> {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        // Check global rate limit first
        if let Some(ref global_limiter) = self.global_limiter {
            if global_limiter.check().is_err() {
                stats.global_rate_limited += 1;
                return Ok(false);
            }
        }

        // Check endpoint-specific rate limit
        if let Some(endpoint_limit) = self.config.endpoint_limits.get(endpoint) {
            let allowed = self
                .check_endpoint_limit(ip, endpoint, endpoint_limit)
                .await?;
            if !allowed {
                stats.endpoint_rate_limited += 1;
                return Ok(false);
            }
        }

        // Check general IP rate limit
        let allowed = self.check_ip_limit(ip).await?;
        if !allowed {
            stats.ip_rate_limited += 1;
            return Ok(false);
        }

        stats.allowed_requests += 1;
        Ok(true)
    }

    async fn check_ip_limit(&self, ip: IpAddr) -> Result<bool> {
        let mut limiters = self.ip_limiters.write().await;

        let limiter = limiters.entry(ip).or_insert_with(|| {
            let quota =
                Quota::per_second(NonZeroU32::new(self.config.requests_per_second).unwrap())
                    .allow_burst(NonZeroU32::new(self.config.burst_capacity).unwrap());
            GovernorRateLimiter::direct(quota)
        });

        Ok(limiter.check().is_ok())
    }

    async fn check_endpoint_limit(
        &self,
        ip: IpAddr,
        endpoint: &str,
        limit: &EndpointLimit,
    ) -> Result<bool> {
        let mut endpoint_limiters = self.endpoint_limiters.write().await;

        let ip_limiters = endpoint_limiters
            .entry(endpoint.to_string())
            .or_insert_with(HashMap::new);

        let limiter = ip_limiters.entry(ip).or_insert_with(|| {
            let quota = Quota::per_second(NonZeroU32::new(limit.requests_per_second).unwrap())
                .allow_burst(NonZeroU32::new(limit.burst_capacity).unwrap());
            GovernorRateLimiter::direct(quota)
        });

        Ok(limiter.check().is_ok())
    }

    /// Clean up old rate limiter entries to prevent memory leaks
    pub async fn cleanup_old_entries(&self) {
        // This is a simplified cleanup - in production, you'd want more sophisticated cleanup
        // based on last access time
        let mut ip_limiters = self.ip_limiters.write().await;
        let mut endpoint_limiters = self.endpoint_limiters.write().await;

        // Keep only the most recent 10000 IP limiters
        if ip_limiters.len() > 10000 {
            let keys_to_remove: Vec<_> = ip_limiters
                .keys()
                .take(ip_limiters.len() - 10000)
                .cloned()
                .collect();
            for key in keys_to_remove {
                ip_limiters.remove(&key);
            }
        }

        // Clean up endpoint limiters
        for (_, ip_limiters) in endpoint_limiters.iter_mut() {
            if ip_limiters.len() > 1000 {
                let keys_to_remove: Vec<_> = ip_limiters
                    .keys()
                    .take(ip_limiters.len() - 1000)
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    ip_limiters.remove(&key);
                }
            }
        }
    }

    /// Get rate limiting statistics
    pub fn get_stats(&self) -> serde_json::Value {
        // This would be async in a real implementation
        serde_json::json!({
            "rate_limiter": "active",
            "note": "Detailed stats would be available in async context"
        })
    }
}

/// Rate limiting statistics
#[derive(Debug, Default)]
struct RateLimitStats {
    pub total_requests: u64,
    pub allowed_requests: u64,
    pub ip_rate_limited: u64,
    pub endpoint_rate_limited: u64,
    pub global_rate_limited: u64,
}

/// Rate limiting errors
#[derive(thiserror::Error, Debug)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for IP")]
    IpRateLimited,
    #[error("Rate limit exceeded for endpoint")]
    EndpointRateLimited,
    #[error("Global rate limit exceeded")]
    GlobalRateLimited,
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);
        assert!(limiter.is_ok());
    }

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        let config = RateLimitConfig {
            requests_per_second: 2,
            burst_capacity: 2,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First two requests should pass
        assert!(limiter.check_rate_limit(ip, "/test").await.unwrap());
        assert!(limiter.check_rate_limit(ip, "/test").await.unwrap());

        // Third request should be rate limited
        assert!(!limiter.check_rate_limit(ip, "/test").await.unwrap());
    }

    #[tokio::test]
    async fn test_endpoint_specific_limits() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config).unwrap();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // /tx endpoint has stricter limits (10 rps)
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(ip, "/tx").await.unwrap());
        }

        // 11th request should be rate limited
        assert!(!limiter.check_rate_limit(ip, "/tx").await.unwrap());
    }

    #[tokio::test]
    async fn test_different_ips() {
        let config = RateLimitConfig {
            requests_per_second: 1,
            burst_capacity: 1,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        // Each IP should have its own limit
        assert!(limiter.check_rate_limit(ip1, "/test").await.unwrap());
        assert!(limiter.check_rate_limit(ip2, "/test").await.unwrap());

        // Both should be rate limited on second request
        assert!(!limiter.check_rate_limit(ip1, "/test").await.unwrap());
        assert!(!limiter.check_rate_limit(ip2, "/test").await.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limit_recovery() {
        let config = RateLimitConfig {
            requests_per_second: 10, // Allow recovery quickly for test
            burst_capacity: 1,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config).unwrap();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Use up the burst capacity
        assert!(limiter.check_rate_limit(ip, "/test").await.unwrap());
        assert!(!limiter.check_rate_limit(ip, "/test").await.unwrap());

        // Wait for rate limit to recover
        sleep(Duration::from_millis(200)).await;

        // Should be able to make requests again
        assert!(limiter.check_rate_limit(ip, "/test").await.unwrap());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config).unwrap();

        // Add some entries
        for i in 0..100 {
            let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, i as u8));
            limiter.check_rate_limit(ip, "/test").await.unwrap();
        }

        // Cleanup should not fail
        limiter.cleanup_old_entries().await;
    }
}
