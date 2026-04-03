use axum::{Json, Router, extract::State, routing::get};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::processes::{ProcessHealth, ProcessStatus, Supervisor};

// ═══════════════════════════════════════════════════════════
// Health API response types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthResponse {
    pub status: AggregateStatus,
    pub service: &'static str,
    pub version: &'static str,
    pub display: String,
    pub processes: Vec<ProcessHealth>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregateStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

// ═══════════════════════════════════════════════════════════
// Shared state — supervisor behind a RwLock
// ═══════════════════════════════════════════════════════════

pub type SharedSupervisor = Arc<RwLock<Supervisor>>;

// ═══════════════════════════════════════════════════════════
// HTTP server
// ═══════════════════════════════════════════════════════════

pub async fn serve(port: u16, supervisor: SharedSupervisor) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(supervisor);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("health server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler(State(sup): State<SharedSupervisor>) -> Json<HealthResponse> {
    let sup = sup.read().await;
    let processes = sup.health();
    let all_healthy = sup.all_healthy();

    let status = if all_healthy {
        AggregateStatus::Healthy
    } else if processes.iter().any(|p| p.status == ProcessStatus::Running) {
        AggregateStatus::Degraded
    } else {
        AggregateStatus::Unhealthy
    };

    Json(HealthResponse {
        status,
        service: "reach-supervisor",
        version: env!("CARGO_PKG_VERSION"),
        display: std::env::var("DISPLAY").unwrap_or_else(|_| ":99".into()),
        processes,
    })
}

async fn metrics_handler() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
