use clap::Parser;

mod cli;
mod commands;

#[tokio::main]
async fn main() {
	let cli = cli::Cli::parse();
	std::process::exit(match &cli.command {
		cli::CliCommand::Co(command) => commands::co::command(&cli, &command).await.unwrap(),
		cli::CliCommand::CoreBuildBuiltin => commands::core_build_builtin::command().await.unwrap(),
		cli::CliCommand::Cbor(command) => commands::cbor::command(command).await.unwrap(),
	})
}
