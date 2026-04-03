use crate::docker::DockerClient;
use clap::Args;

#[derive(Args)]
pub struct VncArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(args: VncArgs) -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    let sandbox = docker.find(&args.target).await?;

    let port = sandbox
        .ports
        .novnc
        .ok_or_else(|| anyhow::anyhow!("noVNC port not mapped"))?;

    let url = format!("http://localhost:{}/vnc.html?autoconnect=true", port);
    println!("Opening {}", url);
    open::that(&url)?;
    Ok(())
}
