use clap::Args;

#[derive(Args)]
pub struct VncArgs {
    /// Sandbox name or container ID
    pub target: String,
}

pub async fn run(_args: VncArgs) -> anyhow::Result<()> {
    todo!("reach vnc")
}
