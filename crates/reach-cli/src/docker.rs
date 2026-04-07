use anyhow::{Context, Result, bail};
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{HostConfig, PortBinding};
use futures::StreamExt;
use std::collections::HashMap;
use std::time::Duration;

// ═══════════════════════════════════════════════════════════
// Sandbox configuration
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
    /// Additional host:container port pairs to publish, beyond the three
    /// built-in ports above. Used for ad-hoc workflows that need to expose
    /// extra services from inside the sandbox — e.g. forwarding Chrome's
    /// remote debugging port (9222) so a host process can drive an agent
    /// browser via CDP. Each tuple is (host_port, container_port).
    pub extra: Vec<(u16, u16)>,
}

impl Default for SandboxPorts {
    fn default() -> Self {
        Self {
            vnc: 5900,
            novnc: 6080,
            health: 8400,
            extra: Vec::new(),
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
            shm_size: 2 * 1024 * 1024 * 1024,
            ports: SandboxPorts::default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Sandbox runtime state
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
    /// Extra (host_port, container_port) pairs published by the user via
    /// `--extra-port`. Empty when no extras were requested.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(u16, u16)>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

// ═══════════════════════════════════════════════════════════
// Labels
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
        labels.insert(Self::CREATED.into(), chrono::Utc::now().to_rfc3339());
        labels.insert(Self::RESOLUTION.into(), config.resolution.to_string());
        labels
    }

    pub fn filter() -> HashMap<String, Vec<String>> {
        let mut filters = HashMap::new();
        filters.insert("label".into(), vec![format!("{}=true", Self::MANAGED)]);
        filters
    }
}

// ═══════════════════════════════════════════════════════════
// Docker client
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

    pub async fn create(&self, config: SandboxConfig) -> Result<Sandbox> {
        let labels = Labels::for_sandbox(&config);

        let port_bindings = {
            let mut map: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
            map.insert(
                "5900/tcp".into(),
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".into()),
                    host_port: Some(config.ports.vnc.to_string()),
                }]),
            );
            map.insert(
                "6080/tcp".into(),
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".into()),
                    host_port: Some(config.ports.novnc.to_string()),
                }]),
            );
            map.insert(
                "8400/tcp".into(),
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".into()),
                    host_port: Some(config.ports.health.to_string()),
                }]),
            );
            for (host_port, container_port) in &config.ports.extra {
                map.insert(
                    format!("{}/tcp", container_port),
                    Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0".into()),
                        host_port: Some(host_port.to_string()),
                    }]),
                );
            }
            map
        };

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            shm_size: Some(config.shm_size as i64),
            ..Default::default()
        };

        let container_config = Config {
            image: Some(config.image.clone()),
            labels: Some(labels),
            host_config: Some(host_config),
            env: Some(vec![
                format!("WIDTH={}", config.resolution.width),
                format!("HEIGHT={}", config.resolution.height),
            ]),
            exposed_ports: Some({
                let mut m = HashMap::new();
                m.insert("5900/tcp".into(), HashMap::new());
                m.insert("6080/tcp".into(), HashMap::new());
                m.insert("8400/tcp".into(), HashMap::new());
                for (_, container_port) in &config.ports.extra {
                    m.insert(format!("{}/tcp", container_port), HashMap::new());
                }
                m
            }),
            ..Default::default()
        };

        let opts = CreateContainerOptions {
            name: &config.name,
            platform: None,
        };

        let resp = self
            .client
            .create_container(Some(opts), container_config)
            .await
            .context("failed to create container")?;

        self.client
            .start_container::<String>(&resp.id, None)
            .await
            .context("failed to start container")?;

        tracing::info!(name = config.name, id = &resp.id[..12], "sandbox created");

        Ok(Sandbox {
            name: config.name,
            container_id: resp.id,
            status: SandboxStatus::Starting,
            image: config.image,
            ports: SandboxPortMapping {
                vnc: Some(config.ports.vnc),
                novnc: Some(config.ports.novnc),
                health: Some(config.ports.health),
                extra: config.ports.extra.clone(),
            },
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn destroy(&self, target: &str) -> Result<()> {
        let sandbox = self.find(target).await?;

        self.client
            .stop_container(&sandbox.container_id, Some(StopContainerOptions { t: 10 }))
            .await
            .context("failed to stop container")?;

        self.client
            .remove_container(
                &sandbox.container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .context("failed to remove container")?;

        tracing::info!(name = sandbox.name, "sandbox destroyed");
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Sandbox>> {
        let opts = ListContainersOptions {
            all: true,
            filters: Labels::filter(),
            ..Default::default()
        };

        let containers = self.client.list_containers(Some(opts)).await?;

        let sandboxes = containers
            .into_iter()
            .map(|c| {
                let labels = c.labels.unwrap_or_default();
                let name = labels
                    .get(Labels::NAME)
                    .cloned()
                    .unwrap_or_else(|| "unknown".into());
                let status = c
                    .state
                    .as_deref()
                    .map(SandboxStatus::from)
                    .unwrap_or(SandboxStatus::Unknown);

                let ports = extract_ports(&c.ports.unwrap_or_default());

                Sandbox {
                    name,
                    container_id: c.id.unwrap_or_default(),
                    status,
                    image: c.image.unwrap_or_default(),
                    ports,
                    created_at: labels.get(Labels::CREATED).cloned().unwrap_or_default(),
                }
            })
            .collect();

        Ok(sandboxes)
    }

    pub async fn find(&self, target: &str) -> Result<Sandbox> {
        let sandboxes = self.list().await?;
        sandboxes
            .into_iter()
            .find(|s| s.name == target || s.container_id.starts_with(target))
            .ok_or_else(|| anyhow::anyhow!("sandbox '{}' not found", target))
    }

    pub async fn exec(&self, target: &str, command: &[String]) -> Result<ExecOutput> {
        let sandbox = self.find(target).await?;
        let cmd: Vec<&str> = command.iter().map(|s| s.as_str()).collect();

        let exec = self
            .client
            .create_exec(
                &sandbox.container_id,
                CreateExecOptions {
                    cmd: Some(cmd),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    env: Some(vec!["DISPLAY=:99"]),
                    ..Default::default()
                },
            )
            .await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } =
            self.client.start_exec(&exec.id, None).await?
        {
            while let Some(Ok(msg)) = output.next().await {
                match msg {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        let inspect = self.client.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(-1);

        Ok(ExecOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    pub async fn screenshot(&self, target: &str) -> Result<Vec<u8>> {
        let out = self
            .exec(
                target,
                &[
                    "bash".into(),
                    "-c".into(),
                    "scrot -z /tmp/_reach_shot.png && base64 /tmp/_reach_shot.png && rm /tmp/_reach_shot.png".into(),
                ],
            )
            .await?;

        if out.exit_code != 0 {
            bail!("screenshot failed: {}", out.stderr);
        }

        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(out.stdout.trim())
            .context("failed to decode screenshot base64")?;

        Ok(bytes)
    }

    pub async fn wait_healthy(&self, target: &str, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if tokio::time::Instant::now() > deadline {
                bail!("timeout waiting for sandbox '{}' to become healthy", target);
            }

            let out = self
                .exec(
                    target,
                    &[
                        "curl".into(),
                        "-sf".into(),
                        "http://localhost:8400/health".into(),
                    ],
                )
                .await;

            if let Ok(result) = out
                && result.exit_code == 0
            {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

fn extract_ports(ports: &[bollard::models::Port]) -> SandboxPortMapping {
    let mut mapping = SandboxPortMapping {
        vnc: None,
        novnc: None,
        health: None,
        extra: Vec::new(),
    };

    for p in ports {
        match p.private_port {
            5900 => mapping.vnc = p.public_port,
            6080 => mapping.novnc = p.public_port,
            8400 => mapping.health = p.public_port,
            other => {
                if let Some(host_port) = p.public_port {
                    mapping.extra.push((host_port, other));
                }
            }
        }
    }

    mapping
}
