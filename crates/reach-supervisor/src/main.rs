mod health;
mod processes;
mod signals;

use anyhow::Context;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "reach_supervisor=info".into()),
        )
        .init();

    tracing::info!("reach-supervisor starting as PID 1");

    // Clean stale X11 locks
    processes::clean_x11_locks()?;

    // Start supervised processes
    let mut supervisor = processes::Supervisor::from_env();
    supervisor
        .start_all()
        .await
        .context("failed to start managed processes")?;

    // Wrap in shared state for the health server
    let shared = Arc::new(RwLock::new(supervisor));

    // Start health + metrics server
    let health_handle = tokio::spawn(health::serve(8400, Arc::clone(&shared)));

    // Wait for shutdown signal
    signals::wait_for_shutdown().await?;

    // Graceful shutdown
    tracing::info!("shutting down");
    shared.write().await.stop_all().await?;
    health_handle.abort();

    Ok(())
}
