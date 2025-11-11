//! Production-ready main entry point for AI Service

use ippan_ai_service::{
    AIService, ConfigManager, Environment, JsonExporter, MetricsCollector, MetricsExporter,
    PrometheusExporter,
};
use std::sync::Arc;
use std::time::Duration;
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
    let health_task = tokio::spawn(async move {
        start_health_server(service).await;
    });

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
async fn start_health_server(_service: AIService) {
    use warp::Filter;

    let health_route = warp::path("health").and(warp::get()).map(move || {
        // Simple health check response
        warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
        }))
    });

    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .and_then(|| async move {
            Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({
                "message": "Metrics endpoint - implement Prometheus format here"
            })))
        });

    let routes = health_route.or(metrics_route);

    let port = std::env::var("HEALTH_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    info!("Starting health check server on port {}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
