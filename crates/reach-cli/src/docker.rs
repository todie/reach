use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StopContainerOptions,
};
use bollard::models::HostConfig;
use std::collections::HashMap;
use std::time::Duration;

// ═══════════════════════════════════════════════════════════
// Sandbox configuration — what the user asks for
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub name: String,
    pub image: String,
    pub resolution: Resolution,
    pub shm_size: u64,
    pub ports: SandboxPorts,
}

#[derive(Debug, Clone)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('x').collect();
        anyhow::ensure!(parts.len() == 2, "resolution must be WxH (e.g., 1280x720)");
        Ok(Self {
            width: parts[0].parse().context("invalid width")?,
            height: parts[1].parse().context("invalid height")?,
        })
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[derive(Debug, Clone)]
pub struct SandboxPorts {
    pub vnc: u16,
    pub novnc: u16,
    pub health: u16,
}

impl Default for SandboxPorts {
    fn default() -> Self {
        Self {
            vnc: 5900,
            novnc: 6080,
            health: 8400,
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            name: "reach".into(),
            image: "reach:latest".into(),
            resolution: Resolution {
                width: 1280,
                height: 720,
            },
            shm_size: 2 * 1024 * 1024 * 1024, // 2GB
            ports: SandboxPorts::default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Sandbox — runtime representation of a running container
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize)]
pub struct Sandbox {
    pub name: String,
    pub container_id: String,
    pub status: SandboxStatus,
    pub image: String,
    pub ports: SandboxPortMapping,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    Running,
    Starting,
    Stopped,
    Unhealthy,
    Unknown,
}

impl From<&str> for SandboxStatus {
    fn from(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "created" | "restarting" => Self::Starting,
            "exited" | "dead" => Self::Stopped,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SandboxPortMapping {
    pub vnc: Option<u16>,
    pub novnc: Option<u16>,
    pub health: Option<u16>,
}

// ═══════════════════════════════════════════════════════════
// Container labels — how we tag and discover reach containers
// ═══════════════════════════════════════════════════════════

pub struct Labels;

impl Labels {
    pub const MANAGED: &str = "reach.sandbox";
    pub const NAME: &str = "reach.name";
    pub const CREATED: &str = "reach.created";
    pub const RESOLUTION: &str = "reach.resolution";

    pub fn for_sandbox(config: &SandboxConfig) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        labels.insert(Self::MANAGED.into(), "true".into());
        labels.insert(Self::NAME.into(), config.name.clone());
        labels.insert(
            Self::CREATED.into(),
            chrono::Utc::now().to_rfc3339(),
        );
        labels.insert(Self::RESOLUTION.into(), config.resolution.to_string());
        labels
    }

    pub fn filter() -> HashMap<String, Vec<String>> {
        let mut filters = HashMap::new();
        filters.insert(
            "label".into(),
            vec![format!("{}=true", Self::MANAGED)],
        );
        filters
    }
}

// ═══════════════════════════════════════════════════════════
// Docker client — trait for testability, impl for bollard
// ═══════════════════════════════════════════════════════════

#[async_trait::async_trait]
pub trait SandboxManager {
    async fn create(&self, config: SandboxConfig) -> Result<Sandbox>;
    async fn destroy(&self, target: &str) -> Result<()>;
    async fn list(&self) -> Result<Vec<Sandbox>>;
    async fn find(&self, target: &str) -> Result<Sandbox>;
    async fn exec(&self, target: &str, command: &[String]) -> Result<ExecOutput>;
    async fn screenshot(&self, target: &str) -> Result<Vec<u8>>;
    async fn wait_healthy(&self, target: &str, timeout: Duration) -> Result<()>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

// ═══════════════════════════════════════════════════════════
// Bollard implementation
// ═══════════════════════════════════════════════════════════

pub struct DockerClient {
    client: bollard::Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self> {
        let client = bollard::Docker::connect_with_local_defaults()?;
        Ok(Self { client })
    }

    pub fn inner(&self) -> &bollard::Docker {
        &self.client
    }
}
