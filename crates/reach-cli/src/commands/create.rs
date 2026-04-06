use clap::Args;
use colored::Colorize;
use reach_cli::config::ReachConfig;
use reach_cli::docker::{DockerClient, ProfileMount, Resolution, SandboxConfig, SandboxPorts};
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

    /// Persist a Chrome profile across sandbox restarts.
    ///
    /// The named profile is stored on the host under
    /// `~/.local/share/reach/profiles/<name>` (overridable via the
    /// `sandbox.profile_dir` config key) and bind-mounted into the
    /// container at `/home/sandbox/.config/google-chrome-profiles/<name>`.
    /// Pass the same name to `page_text` / `auth_handoff` via
    /// `use_profile` to reuse the session.
    #[arg(long, value_name = "NAME")]
    pub persist_profile: Option<String>,
}

pub async fn run(args: CreateArgs) -> anyhow::Result<()> {
    let cfg = ReachConfig::load();
    let resolution = Resolution::parse(&args.resolution)?;

    let profile = args.persist_profile.as_ref().map(|name| {
        let host_path = ProfileMount::host_path_for(&cfg.sandbox.resolved_profile_dir(), name);
        ProfileMount {
            name: name.clone(),
            host_path,
            container_path: ProfileMount::container_path_for(name),
        }
    });

    let config = SandboxConfig {
        name: args.name.clone(),
        image: args.image.unwrap_or(cfg.sandbox.image.clone()),
        resolution,
        shm_size: cfg.sandbox.shm_size,
        ports: SandboxPorts {
            vnc: args.vnc_port.unwrap_or(cfg.sandbox.vnc_port),
            novnc: args.novnc_port.unwrap_or(cfg.sandbox.novnc_port),
            health: args.health_port.unwrap_or(cfg.sandbox.health_port),
        },
        profile,
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

    if let Some(name) = &args.persist_profile {
        let host = ProfileMount::host_path_for(&cfg.sandbox.resolved_profile_dir(), name);
        println!(
            "  {} {}  {} {}",
            "\u{2713}".green(),
            "Profile   ".dimmed(),
            name,
            format!("({})", host.display()).dimmed()
        );
    }

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

    println!();
    println!(
        "  Sandbox {} ready.",
        format!("\"{}\"", sandbox.name).green().bold()
    );
    println!();
    Ok(())
}
