use actix_web::{get, HttpResponse, Responder};
use crate::node::{IppanNode, NodeHealth, NodeMetrics};

#[get("/health")]
pub async fn health_check(node: web::Data<Arc<RwLock<IppanNode>>>) -> impl Responder {
    let node_guard = node.read().await;
    let health = node_guard.health_check().await;
    
    HttpResponse::Ok().json(health)
}

#[get("/health/detailed")]
pub async fn detailed_health_check(node: web::Data<Arc<RwLock<IppanNode>>>) -> impl Responder {
    let node_guard = node.read().await;
    let metrics = node_guard.collect_metrics().await;
    
    HttpResponse::Ok().json(metrics)
}

#[get("/health/ready")]
pub async fn readiness_check(node: web::Data<Arc<RwLock<IppanNode>>>) -> impl Responder {
    let node_guard = node.read().await;
    let health = node_guard.health_check().await;
    
    match health.status {
        NodeStatus::Healthy | NodeStatus::Degraded => {
            HttpResponse::Ok().json(serde_json::json!({
                "status": "ready",
                "message": "Node is ready to serve requests"
            }))
        }
        _ => {
            HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "status": "not_ready",
                "message": "Node is not ready to serve requests"
            }))
        }
    }
}

#[get("/health/live")]
pub async fn liveness_check() -> impl Responder {
    // Simple liveness check - if we can respond, the service is alive
    HttpResponse::Ok().json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now()
    }))
} 