use crate::config::ReachConfig;
use crate::docker::{DockerClient, Resolution, SandboxConfig, SandboxPorts};
use clap::Args;
use std::time::Duration;

#[derive(Args)]
pub struct CreateArgs {
    /// Name for the sandbox container
    #[arg(long, default_value = "reach")]
    pub name: String,

    /// Display resolution (WxH)
    #[arg(long, default_value = "1280x720")]
    pub resolution: String,

    /// Docker image to use
    #[arg(long)]
    pub image: Option<String>,

    /// VNC port
    #[arg(long)]
    pub vnc_port: Option<u16>,

    /// noVNC port
    #[arg(long)]
    pub novnc_port: Option<u16>,

    /// Health API port
    #[arg(long)]
    pub health_port: Option<u16>,

    /// Skip waiting for health check
    #[arg(long)]
    pub no_wait: bool,
}

pub async fn run(args: CreateArgs) -> anyhow::Result<()> {
    let cfg = ReachConfig::load();
    let resolution = Resolution::parse(&args.resolution)?;

    let config = SandboxConfig {
        name: args.name.clone(),
        image: args.image.unwrap_or(cfg.sandbox.image),
        resolution,
        shm_size: cfg.sandbox.shm_size,
        ports: SandboxPorts {
            vnc: args.vnc_port.unwrap_or(cfg.sandbox.vnc_port),
            novnc: args.novnc_port.unwrap_or(cfg.sandbox.novnc_port),
            health: args.health_port.unwrap_or(cfg.sandbox.health_port),
        },
    };

    let docker = DockerClient::new()?;
    let sandbox = docker.create(config).await?;

    println!("Creating sandbox \"{}\"...", sandbox.name);
    println!("  Container: {}", &sandbox.container_id[..12]);
    println!("  Image:     {}", sandbox.image);

    if !args.no_wait {
        print!("  Waiting for health...");
        docker
            .wait_healthy(&args.name, Duration::from_secs(30))
            .await?;
        println!(" ready.");
    }

    if let Some(p) = sandbox.ports.novnc {
        println!("  VNC:       http://localhost:{}", p);
    }
    if let Some(p) = sandbox.ports.health {
        println!("  Health:    http://localhost:{}/health", p);
    }

    println!("Sandbox \"{}\" ready.", sandbox.name);
    Ok(())
}
