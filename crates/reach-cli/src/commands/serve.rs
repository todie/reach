use clap::Args;

#[derive(Args)]
pub struct ServeArgs {
    /// Port for the MCP SSE server
    #[arg(long, default_value = "4200")]
    pub port: u16,
}

pub async fn run(_args: ServeArgs) -> anyhow::Result<()> {
    todo!("reach serve")
}
