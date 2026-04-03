use clap::Args;

#[derive(Args)]
pub struct ExecArgs {
    /// Sandbox name or container ID
    pub target: String,

    /// Command to execute
    #[arg(last = true)]
    pub command: Vec<String>,
}

pub async fn run(_args: ExecArgs) -> anyhow::Result<()> {
    todo!("reach exec")
}
