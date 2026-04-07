use clap::Args;
use reach_cli::docker::DockerClient;

#[derive(Args)]
pub struct ExecArgs {
    /// Sandbox name or container ID
    pub target: String,

    /// Command to execute
    #[arg(last = true)]
    pub command: Vec<String>,
}

pub async fn run(args: ExecArgs) -> anyhow::Result<()> {
    anyhow::ensure!(!args.command.is_empty(), "no command specified");

    let docker = DockerClient::new()?;
    let output = docker.exec(&args.target, &args.command).await?;

    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }

    std::process::exit(output.exit_code as i32);
}
