use clap::Parser;
use commands::cores_build::cores_build;

mod cli;
mod commands;

#[tokio::main]
async fn main() {
	let cli = cli::Cli::parse();
	std::process::exit(match cli.command {
		cli::CliCommand::CoresBuild => cores_build().await.unwrap(),
	})
}
