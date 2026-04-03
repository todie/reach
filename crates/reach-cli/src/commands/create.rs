use clap::Args;

#[derive(Args)]
pub struct CreateArgs {
    /// Name for the sandbox container
    #[arg(long, default_value = "reach")]
    pub name: String,

    /// Display resolution (WxH)
    #[arg(long, default_value = "1280x720")]
    pub resolution: String,
}

pub async fn run(_args: CreateArgs) -> anyhow::Result<()> {
    todo!("reach create")
}
