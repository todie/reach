use clap::Args;
use colored::Colorize;
use reach_cli::config::ReachConfig;
use reach_cli::docker::{DockerClient, Resolution, SandboxConfig, SandboxPorts};
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

    /// Publish an additional port from the sandbox to the host.
    ///
    /// Format: `HOST:CONTAINER` or `PORT` (same on both sides). Repeat the
    /// flag to publish more than one. Example: `--extra-port 9222:9222`
    /// exposes Chrome's CDP debug port so a host process can drive a
    /// browser inside the sandbox.
    #[arg(long = "extra-port", value_name = "HOST:CONTAINER", value_parser = parse_port_pair)]
    pub extra_ports: Vec<(u16, u16)>,

    /// Skip waiting for health check
    #[arg(long)]
    pub no_wait: bool,
}

/// Parse a `HOST:CONTAINER` port pair, or a single `PORT` shorthand for
/// `PORT:PORT`. Returns an error for malformed input or out-of-range numbers.
fn parse_port_pair(s: &str) -> Result<(u16, u16), String> {
    if let Some((h, c)) = s.split_once(':') {
        let host: u16 = h.parse().map_err(|_| format!("invalid host port {h:?}"))?;
        let container: u16 = c
            .parse()
            .map_err(|_| format!("invalid container port {c:?}"))?;
        Ok((host, container))
    } else {
        let p: u16 = s.parse().map_err(|_| format!("invalid port {s:?}"))?;
        Ok((p, p))
    }
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
            extra: args.extra_ports.clone(),
        },
    };

    let docker = DockerClient::new()?;
    let sandbox = docker.create(config).await?;

    println!();
    println!("  {}", "reach create".bold());
    println!("  {}", "\u{2500}".repeat(28).dimmed());
    println!(
        "  {} {}  {}",
        "\u{2713}".green(),
        "Container ".dimmed(),
        &sandbox.container_id[..12]
    );
    println!(
        "  {} {}  {}",
        "\u{2713}".green(),
        "Image     ".dimmed(),
        sandbox.image
    );
    println!(
        "  {} {}  {}",
        "\u{2713}".green(),
        "Resolution".dimmed(),
        args.resolution
    );

    if !args.no_wait {
        print!("  \u{2819} {}", "Waiting for health...".dimmed());
        docker
            .wait_healthy(&args.name, Duration::from_secs(30))
            .await?;
        print!("\r");
        println!("  {} Healthy", "\u{2713}".green());
    }

    println!();
    if let Some(p) = sandbox.ports.novnc {
        println!(
            "    {}     {}",
            "VNC:".bold(),
            format!("http://localhost:{}", p).cyan()
        );
    }
    if let Some(p) = sandbox.ports.health {
        println!(
            "    {}  {}",
            "Health:".bold(),
            format!("http://localhost:{}/health", p).cyan()
        );
    }
    for (host_port, container_port) in &sandbox.ports.extra {
        println!(
            "    {}    localhost:{} -> {}/tcp",
            "Extra:".bold(),
            host_port.to_string().cyan(),
            container_port.to_string().cyan()
        );
    }

    println!();
    println!(
        "  Sandbox {} ready.",
        format!("\"{}\"", sandbox.name).green().bold()
    );
    println!();
    Ok(())
}
