use clap::Args;

#[derive(Args)]
pub struct ConnectArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(_args: ConnectArgs) -> anyhow::Result<()> {
    todo!("reach connect")
}
