pub mod connect;
pub mod create;
pub mod destroy;
pub mod exec;
pub mod list;
pub mod screenshot;
pub mod serve;
pub mod vnc;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Create a new sandbox container
    Create(create::CreateArgs),
    /// Destroy a sandbox container
    Destroy(destroy::DestroyArgs),
    /// List running sandbox containers
    List,
    /// Attach MCP stdio bridge to a sandbox
    Connect(connect::ConnectArgs),
    /// Run a command inside a sandbox
    Exec(exec::ExecArgs),
    /// Start MCP SSE server proxying to sandbox(es)
    Serve(serve::ServeArgs),
    /// Open noVNC in browser
    Vnc(vnc::VncArgs),
    /// Capture a screenshot from a sandbox
    Screenshot(screenshot::ScreenshotArgs),
}

pub async fn run(cmd: Command) -> anyhow::Result<()> {
    match cmd {
        Command::Create(args) => create::run(args).await,
        Command::Destroy(args) => destroy::run(args).await,
        Command::List => list::run().await,
        Command::Connect(args) => connect::run(args).await,
        Command::Exec(args) => exec::run(args).await,
        Command::Serve(args) => serve::run(args).await,
        Command::Vnc(args) => vnc::run(args).await,
        Command::Screenshot(args) => screenshot::run(args).await,
    }
}
