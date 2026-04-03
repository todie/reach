use crate::docker::DockerClient;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct DestroyArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(args: DestroyArgs) -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    docker.destroy(&args.target).await?;
    println!(
        "{} {}",
        "\u{2717}".red(),
        format!("Sandbox \"{}\" destroyed.", args.target).dimmed()
    );
    Ok(())
}
