use clap::Args;

#[derive(Args)]
pub struct DestroyArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(_args: DestroyArgs) -> anyhow::Result<()> {
    todo!("reach destroy")
}
