use reach_cli::docker::{DockerClient, SandboxStatus};
use colored::Colorize;

pub async fn run() -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    let sandboxes = docker.list().await?;

    if sandboxes.is_empty() {
        println!("{}", "No reach sandboxes running.".dimmed());
        return Ok(());
    }

    println!(
        "{:<16} {:<14} {:<10} {:>8} {:>8} {:>8}",
        "NAME".bold().cyan(),
        "CONTAINER".bold().cyan(),
        "STATUS".bold().cyan(),
        "VNC".bold().cyan(),
        "NOVNC".bold().cyan(),
        "HEALTH".bold().cyan(),
    );

    for sb in &sandboxes {
        let status_str = format!("{:?}", sb.status).to_lowercase();
        let status_colored = match sb.status {
            SandboxStatus::Running => status_str.green().to_string(),
            SandboxStatus::Stopped => status_str.red().to_string(),
            SandboxStatus::Starting => status_str.yellow().to_string(),
            SandboxStatus::Unhealthy => status_str.red().to_string(),
            SandboxStatus::Unknown => status_str.dimmed().to_string(),
        };

        println!(
            "{:<16} {:<14} {:<10} {:>8} {:>8} {:>8}",
            sb.name,
            if sb.container_id.len() >= 12 {
                &sb.container_id[..12]
            } else {
                &sb.container_id
            },
            status_colored,
            sb.ports.vnc.map(|p| p.to_string()).unwrap_or("-".into()),
            sb.ports.novnc.map(|p| p.to_string()).unwrap_or("-".into()),
            sb.ports.health.map(|p| p.to_string()).unwrap_or("-".into()),
        );
    }

    Ok(())
}
