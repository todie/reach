use crate::docker::DockerClient;
use clap::Args;

#[derive(Args)]
pub struct DestroyArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(args: DestroyArgs) -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    docker.destroy(&args.target).await?;
    println!("Sandbox \"{}\" destroyed.", args.target);
    Ok(())
}
