use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════
// CLI configuration — loaded from ~/.config/reach/config.toml
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct ReachConfig {
    pub sandbox: SandboxDefaults,
    pub server: ServerConfig,
    pub docker: DockerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxDefaults {
    /// Default Docker image
    pub image: String,
    /// Default display resolution
    pub resolution: String,
    /// Shared memory size in bytes
    pub shm_size: u64,
    /// Default VNC port
    pub vnc_port: u16,
    /// Default noVNC port
    pub novnc_port: u16,
    /// Default health API port
    pub health_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// MCP SSE server port
    pub port: u16,
    /// Bind address
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct DockerConfig {
    /// Docker socket path (empty = auto-detect)
    pub socket: String,
}

// ═══════════════════════════════════════════════════════════
// Defaults
// ═══════════════════════════════════════════════════════════

impl Default for SandboxDefaults {
    fn default() -> Self {
        Self {
            image: "reach:latest".into(),
            resolution: "1280x720".into(),
            shm_size: 2 * 1024 * 1024 * 1024,
            vnc_port: 5900,
            novnc_port: 6080,
            health_port: 8400,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 4200,
            host: "127.0.0.1".into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Loading
// ═══════════════════════════════════════════════════════════

impl ReachConfig {
    pub fn config_path() -> PathBuf {
        dirs().join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }
}

fn dirs() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".config")
        });
    base.join("reach")
}
