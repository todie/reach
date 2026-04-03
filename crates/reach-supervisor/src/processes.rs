use anyhow::Result;
use std::path::Path;

/// Managed process definitions and supervision logic.
pub struct Supervisor {
    // Will hold child process handles
}

impl Supervisor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start_all(&mut self) -> Result<()> {
        todo!("start Xvfb, openbox, x11vnc, noVNC")
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        todo!("graceful shutdown of all children")
    }
}

/// Remove stale X11 lock files that prevent Xvfb from starting.
pub fn clean_x11_locks() -> Result<()> {
    let lock = Path::new("/tmp/.X99-lock");
    if lock.exists() {
        std::fs::remove_file(lock)?;
        tracing::info!("removed stale X11 lock: {}", lock.display());
    }
    Ok(())
}
