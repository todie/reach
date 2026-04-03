use clap::Args;

#[derive(Args)]
pub struct ScreenshotArgs {
    /// Sandbox name or container ID
    pub target: String,

    /// Output file path
    #[arg(long, short)]
    pub output: Option<String>,
}

pub async fn run(_args: ScreenshotArgs) -> anyhow::Result<()> {
    todo!("reach screenshot")
}
