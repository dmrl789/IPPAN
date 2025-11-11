//! Production-ready main entry point for AI Service

use ippan_ai_service::{
    service::ServiceStatus, AIService, ConfigManager, Environment, JsonExporter, MetricsCollector,
    MetricsExporter, MetricsSnapshot, PrometheusExporter,
};
use ippan_security::rate_limiter::EndpointLimit;
use ippan_security::{RateLimitConfig, RateLimitStatsSnapshot, RateLimiter};
use std::fmt::Write as _;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::signal;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging()?;

    info!("Starting IPPAN AI Service v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config_manager = ConfigManager::new().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    let config = config_manager.get_config().clone();
    let environment = config_manager.get_environment().clone();

    info!("Running in {:?} environment", environment);

    // Initialize metrics collector
    let metrics_collector = Arc::new(MetricsCollector::new());

    // Initialize rate limiter
    let rate_limit_config = load_rate_limit_config();
    let rate_limiter = Arc::new(RateLimiter::new(rate_limit_config).map_err(|e| {
        error!("Failed to initialize rate limiter: {}", e);
        e
    })?);

    // Initialize metrics exporters based on environment
    let mut exporters: Vec<Box<dyn MetricsExporter + Send + Sync>> = Vec::new();

    if environment == Environment::Production {
        let prometheus_endpoint = std::env::var("PROMETHEUS_ENDPOINT")
            .unwrap_or_else(|_| "http://prometheus:9090/api/v1/write".to_string());

        if prometheus_endpoint.trim().is_empty() {
            warn!("PROMETHEUS_ENDPOINT is empty; Prometheus exporter disabled");
        } else {
            info!(
                "Configuring Prometheus exporter with endpoint {}",
                prometheus_endpoint
            );
            exporters.push(Box::new(PrometheusExporter::new(
                prometheus_endpoint.trim().to_string(),
            )));
        }
    }

    let json_endpoint = std::env::var("JSON_EXPORTER_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8080/metrics".to_string());
    if json_endpoint.trim().is_empty() {
        warn!("JSON_EXPORTER_ENDPOINT is empty; JSON exporter disabled");
    } else {
        exporters.push(Box::new(JsonExporter::new(
            json_endpoint.trim().to_string(),
        )));
    }

    // Create AI Service
    let mut service = AIService::new(config).map_err(|e| {
        error!("Failed to create AI Service: {}", e);
        e
    })?;

    // Start the service
    service.start().await.map_err(|e| {
        error!("Failed to start AI Service: {}", e);
        e
    })?;

    info!("AI Service started successfully");

    let service_status = Arc::new(RwLock::new(service.get_status()));

    // Start metrics collection task
    let metrics_task = {
        let metrics_collector = metrics_collector.clone();
        let exporters = exporters;
        let interval_seconds = std::env::var("MONITORING_INTERVAL")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(30);

        info!("Metrics collection interval set to {}s", interval_seconds);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_seconds));
            loop {
                interval.tick().await;

                let snapshot = metrics_collector.get_snapshot();
                debug!("Metrics snapshot: {:?}", snapshot);

                for exporter in &exporters {
                    if let Err(e) = exporter.export_metrics(&snapshot) {
                        warn!("Failed to export metrics: {}", e);
                    }
                }
            }
        })
    };

    // Start health check server
    let health_task = {
        let metrics_collector = metrics_collector.clone();
        let rate_limiter = rate_limiter.clone();
        let service_status = service_status.clone();
        tokio::spawn(async move {
            start_health_server(service_status, metrics_collector, rate_limiter).await;
        })
    };

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown
    info!("Shutting down AI Service...");

    if let Err(e) = service.stop().await {
        warn!("Failed to stop AI Service cleanly: {}", e);
    } else if let Ok(mut status) = service_status.write() {
        status.is_running = false;
    }

    // Graceful shutdown complete
    info!("Service shutdown initiated");

    // Cancel background tasks
    metrics_task.abort();
    health_task.abort();

    info!("AI Service stopped gracefully");
    Ok(())
}

/// Initialize logging based on environment
fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let env = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    // Simple logging setup
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(env)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

/// Start health check HTTP server
async fn start_health_server(
    status: Arc<RwLock<ServiceStatus>>,
    metrics_collector: Arc<MetricsCollector>,
    rate_limiter: Arc<RateLimiter>,
) {
    use warp::Filter;

    let health_status = status.clone();
    let health_metrics = metrics_collector.clone();
    let health_rate_limiter = rate_limiter.clone();

    let health_route = warp::path("health")
        .and(warp::get())
        .and(warp::filters::addr::remote())
        .and_then(move |remote: Option<std::net::SocketAddr>| {
            let status = health_status.clone();
            let metrics = health_metrics.clone();
            let rate_limiter = health_rate_limiter.clone();
            async move {
                let ip = remote
                    .map(|addr| addr.ip())
                    .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

                if !rate_limiter
                    .check_rate_limit(ip, "/health")
                    .await
                    .unwrap_or(true)
                {
                    metrics.record_error();
                    metrics.record_request(false, 0);
                    return Err(warp::reject::custom(RateLimited));
                }

                metrics.record_request(true, 0);
                metrics.record_service_request("health");

                let status_snapshot =
                    status
                        .read()
                        .map(|guard| guard.clone())
                        .unwrap_or_else(|_| ServiceStatus {
                            is_running: false,
                            llm_enabled: false,
                            analytics_enabled: false,
                            monitoring_enabled: false,
                            smart_contracts_enabled: false,
                            version: env!("CARGO_PKG_VERSION").to_string(),
                        });
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({
                    "status": if status_snapshot.is_running { "running" } else { "stopped" },
                    "version": env!("CARGO_PKG_VERSION"),
                    "timestamp": timestamp,
                    "service": status_snapshot,
                })))
            }
        });

    let metrics_status = status.clone();
    let metrics_rate_limiter = rate_limiter.clone();
    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .and(warp::filters::addr::remote())
        .and_then(move |remote: Option<std::net::SocketAddr>| {
            let metrics = metrics_collector.clone();
            let rate_limiter = metrics_rate_limiter.clone();
            let service_status = metrics_status.clone();
            async move {
                let ip = remote
                    .map(|addr| addr.ip())
                    .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

                if !rate_limiter
                    .check_rate_limit(ip, "/metrics")
                    .await
                    .unwrap_or(true)
                {
                    metrics.record_error();
                    metrics.record_request(false, 0);
                    return Err(warp::reject::custom(RateLimited));
                }

                metrics.record_request(true, 0);
                metrics.record_service_request("metrics");

                let snapshot = metrics.get_snapshot();
                let rate_limit_stats = rate_limiter.stats_snapshot().await;
                let status_snapshot = service_status
                    .read()
                    .map(|guard| guard.clone())
                    .unwrap_or_else(|_| ServiceStatus {
                        is_running: false,
                        llm_enabled: false,
                        analytics_enabled: false,
                        monitoring_enabled: false,
                        smart_contracts_enabled: false,
                        version: env!("CARGO_PKG_VERSION").to_string(),
                    });

                let body =
                    render_prometheus_metrics(&snapshot, &rate_limit_stats, &status_snapshot);
                Ok::<_, warp::Rejection>(warp::reply::with_header(
                    body,
                    "Content-Type",
                    "text/plain; version=0.0.4",
                ))
            }
        });

    let routes = health_route.or(metrics_route).recover(handle_rejection);

    let port = std::env::var("HEALTH_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    info!("Starting health check server on port {}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

#[derive(Debug)]
struct RateLimited;

impl warp::reject::Reject for RateLimited {}

async fn handle_rejection(
    err: warp::Rejection,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    use warp::http::StatusCode;

    if err.find::<RateLimited>().is_some() {
        let body = serde_json::json!({
            "error": "rate_limited",
            "message": "Too many requests"
        });
        Ok(warp::reply::with_status(
            warp::reply::json(&body),
            StatusCode::TOO_MANY_REQUESTS,
        ))
    } else {
        let body = serde_json::json!({
            "error": "internal_server_error"
        });
        Ok(warp::reply::with_status(
            warp::reply::json(&body),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

fn load_rate_limit_config() -> RateLimitConfig {
    let mut config = RateLimitConfig::default();

    if let Some(rps) = env_u32("AI_RATE_LIMIT_RPS") {
        config.requests_per_second = rps.max(1);
    }

    if let Some(burst) = env_u32("AI_RATE_LIMIT_BURST") {
        config.burst_capacity = burst.max(1);
    }

    config.global_requests_per_second = env_u32("AI_RATE_LIMIT_GLOBAL_RPS");

    let health_limit = env_u32("AI_HEALTH_RATE_LIMIT_RPS").unwrap_or(10).max(1);
    let health_burst = env_u32("AI_HEALTH_RATE_LIMIT_BURST").unwrap_or(20).max(1);

    config.endpoint_limits.insert(
        "/health".to_string(),
        EndpointLimit {
            requests_per_second: health_limit,
            burst_capacity: health_burst,
        },
    );

    let metrics_limit = env_u32("AI_METRICS_RATE_LIMIT_RPS").unwrap_or(5).max(1);
    let metrics_burst = env_u32("AI_METRICS_RATE_LIMIT_BURST").unwrap_or(10).max(1);

    config.endpoint_limits.insert(
        "/metrics".to_string(),
        EndpointLimit {
            requests_per_second: metrics_limit,
            burst_capacity: metrics_burst,
        },
    );

    config
}

fn env_u32(key: &str) -> Option<u32> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
}

fn render_prometheus_metrics(
    snapshot: &MetricsSnapshot,
    rate_limit: &RateLimitStatsSnapshot,
    status: &ServiceStatus,
) -> String {
    let mut output = String::new();

    writeln!(
        &mut output,
        "# HELP ai_service_uptime_seconds Uptime of the AI service in seconds"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_uptime_seconds gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_uptime_seconds {}",
        snapshot.uptime_seconds
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_total_requests Total requests handled by the AI service"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_total_requests counter").unwrap();
    writeln!(
        &mut output,
        "ai_service_total_requests {}",
        snapshot.total_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_successful_requests Successful requests"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_successful_requests counter").unwrap();
    writeln!(
        &mut output,
        "ai_service_successful_requests {}",
        snapshot.successful_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_failed_requests Failed requests"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_failed_requests counter").unwrap();
    writeln!(
        &mut output,
        "ai_service_failed_requests {}",
        snapshot.failed_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_error_count Total errors recorded by the AI service"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_error_count counter").unwrap();
    writeln!(
        &mut output,
        "ai_service_error_count {}",
        snapshot.error_count
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_active_connections Active client connections"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_active_connections gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_active_connections {}",
        snapshot.active_connections
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_cpu_usage_percent CPU usage percentage"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_cpu_usage_percent gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_cpu_usage_percent {}",
        snapshot.cpu_usage_percent
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_memory_usage_bytes Memory usage in bytes"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_memory_usage_bytes gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_memory_usage_bytes {}",
        snapshot.memory_usage_bytes
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_avg_request_duration_ms Average request duration in milliseconds"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_avg_request_duration_ms gauge"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_avg_request_duration_ms {}",
        snapshot.avg_duration_ms
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_success_rate Success rate of handled requests"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_success_rate gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_success_rate {}",
        snapshot.success_rate
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_health_requests_total Health endpoint requests"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_health_requests_total counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_health_requests_total {}",
        snapshot.health_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_metrics_requests_total Metrics endpoint requests"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_metrics_requests_total counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_metrics_requests_total {}",
        snapshot.metrics_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_rate_limit_total_requests Total requests observed by the rate limiter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_rate_limit_total_requests counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_rate_limit_total_requests {}",
        rate_limit.total_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_rate_limit_allowed_requests Requests allowed through the rate limiter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_rate_limit_allowed_requests counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_rate_limit_allowed_requests {}",
        rate_limit.allowed_requests
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_rate_limit_ip_blocked Requests blocked by IP rate limiting"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_rate_limit_ip_blocked counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_rate_limit_ip_blocked {}",
        rate_limit.ip_rate_limited
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_rate_limit_endpoint_blocked Requests blocked by endpoint-specific limits"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_rate_limit_endpoint_blocked counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_rate_limit_endpoint_blocked {}",
        rate_limit.endpoint_rate_limited
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_rate_limit_global_blocked Requests blocked by global limits"
    )
    .unwrap();
    writeln!(
        &mut output,
        "# TYPE ai_service_rate_limit_global_blocked counter"
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_rate_limit_global_blocked {}",
        rate_limit.global_rate_limited
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_up Whether the AI service is reporting as running (1=yes)"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_up gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_up {}",
        if status.is_running { 1 } else { 0 }
    )
    .unwrap();

    writeln!(
        &mut output,
        "# HELP ai_service_feature_enabled Feature enablement status"
    )
    .unwrap();
    writeln!(&mut output, "# TYPE ai_service_feature_enabled gauge").unwrap();
    writeln!(
        &mut output,
        "ai_service_feature_enabled{{feature=\"llm\"}} {}",
        if status.llm_enabled { 1 } else { 0 }
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_feature_enabled{{feature=\"analytics\"}} {}",
        if status.analytics_enabled { 1 } else { 0 }
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_feature_enabled{{feature=\"monitoring\"}} {}",
        if status.monitoring_enabled { 1 } else { 0 }
    )
    .unwrap();
    writeln!(
        &mut output,
        "ai_service_feature_enabled{{feature=\"smart_contracts\"}} {}",
        if status.smart_contracts_enabled { 1 } else { 0 }
    )
    .unwrap();

    output
}
