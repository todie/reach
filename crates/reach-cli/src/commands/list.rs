use crate::docker::DockerClient;

pub async fn run() -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    let sandboxes = docker.list().await?;

    if sandboxes.is_empty() {
        println!("No reach sandboxes running.");
        return Ok(());
    }

    println!(
        "{:<16} {:<14} {:<10} {:<8} {:<8} {:<8}",
        "NAME", "CONTAINER", "STATUS", "VNC", "NOVNC", "HEALTH"
    );

    for sb in &sandboxes {
        println!(
            "{:<16} {:<14} {:<10} {:<8} {:<8} {:<8}",
            sb.name,
            if sb.container_id.len() >= 12 {
                &sb.container_id[..12]
            } else {
                &sb.container_id
            },
            format!("{:?}", sb.status).to_lowercase(),
            sb.ports.vnc.map(|p| p.to_string()).unwrap_or("-".into()),
            sb.ports.novnc.map(|p| p.to_string()).unwrap_or("-".into()),
            sb.ports.health.map(|p| p.to_string()).unwrap_or("-".into()),
        );
    }

    Ok(())
}
