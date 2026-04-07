use clap::Args;
use colored::Colorize;
use reach_cli::docker::DockerClient;
use std::io::Write;

#[derive(Args)]
pub struct ScreenshotArgs {
    /// Sandbox name or container ID
    pub target: String,

    /// Output file path (default: stdout as base64)
    #[arg(long, short)]
    pub output: Option<String>,
}

pub async fn run(args: ScreenshotArgs) -> anyhow::Result<()> {
    let docker = DockerClient::new()?;
    let png_bytes = docker.screenshot(&args.target).await?;

    match args.output {
        Some(path) => {
            std::fs::write(&path, &png_bytes)?;
            println!(
                "{} Screenshot saved to {} {}",
                "\u{2713}".green(),
                path.bold(),
                format!("({} bytes)", png_bytes.len()).dimmed()
            );
        }
        None => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            std::io::stdout().write_all(encoded.as_bytes())?;
            std::io::stdout().write_all(b"\n")?;
        }
    }

    Ok(())
}
