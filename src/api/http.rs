//! HTTP server module
//! 
//! Provides HTTP API endpoints for the IPPAN node.

use crate::{api::{ApiState, create_router}, error::IppanError, Result};
use axum::{
    http::{HeaderValue, Method},
    Server,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

/// HTTP server
pub struct HttpServer {
    /// Server configuration
    config: crate::api::ApiConfig,
    /// API state
    state: ApiState,
    /// Request counter
    request_count: Arc<AtomicU64>,
    /// Active connections counter
    active_connections: Arc<AtomicU64>,
    /// Server handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(config: crate::api::ApiConfig, state: ApiState) -> Self {
        Self {
            config,
            state,
            request_count: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            server_handle: None,
        }
    }
    
    /// Start the HTTP server
    pub async fn start(&mut self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
        
        // Create router with middleware
        let mut router = create_router(self.state.clone());
        
        // Add CORS if enabled
        if self.config.cors_enabled {
            let cors = CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(["*"]);
            
            router = router.layer(cors);
        }
        
        // Add request counting middleware
        let request_count = self.request_count.clone();
        let active_connections = self.active_connections.clone();
        
        router = router.layer(tower::ServiceBuilder::new()
            .layer(tower::layer::util::MapRequestLayer::new(move |req: axum::http::Request<_>| {
                request_count.fetch_add(1, Ordering::Relaxed);
                req
            }))
            .layer(tower::layer::util::MapResponseLayer::new(move |res: axum::http::Response<_>| {
                // Track active connections
                if let Some(connection) = res.extensions().get::<ConnectionTracker>() {
                    active_connections.fetch_add(1, Ordering::Relaxed);
                }
                res
            })));
        
        info!("Starting HTTP server on {}", addr);
        
        // Start server in background task
        let server_handle = tokio::spawn(async move {
            match Server::bind(&addr).serve(router.into_make_service()).await {
                Ok(_) => info!("HTTP server stopped"),
                Err(e) => error!("HTTP server error: {}", e),
            }
        });
        
        self.server_handle = Some(server_handle);
        
        info!("HTTP server started successfully on {}", addr);
        Ok(())
    }
    
    /// Stop the HTTP server
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            if let Err(e) = handle.await {
                if !e.is_cancelled() {
                    warn!("HTTP server shutdown error: {}", e);
                }
            }
        }
        
        info!("HTTP server stopped");
        Ok(())
    }
    
    /// Get request count
    pub async fn get_request_count(&self) -> Result<u64> {
        Ok(self.request_count.load(Ordering::Relaxed))
    }
    
    /// Get active connections
    pub async fn get_active_connections(&self) -> Result<u64> {
        Ok(self.active_connections.load(Ordering::Relaxed))
    }
    
    /// Get server statistics
    pub async fn get_stats(&self) -> Result<HttpStats> {
        Ok(HttpStats {
            request_count: self.request_count.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            port: self.config.http_port,
            cors_enabled: self.config.cors_enabled,
        })
    }
}

/// HTTP statistics
#[derive(Debug, Clone)]
pub struct HttpStats {
    /// Total requests served
    pub request_count: u64,
    /// Active connections
    pub active_connections: u64,
    /// Server port
    pub port: u16,
    /// CORS enabled
    pub cors_enabled: bool,
}

/// Connection tracker for monitoring active connections
#[derive(Debug)]
pub struct ConnectionTracker;

impl ConnectionTracker {
    /// Create a new connection tracker
    pub fn new() -> Self {
        Self
    }
}
