use clap::Parser;

mod cli;
mod commands;
pub mod library;

fn main() {
	let result = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async { app_main().await });
	std::process::exit(result.unwrap());
}

async fn app_main() -> anyhow::Result<exitcode::ExitCode> {
	let cli = cli::Cli::parse();
	std::process::exit(match &cli.command {
		cli::CliCommand::Co(command) => commands::co::command(&cli, &command).await?,
		cli::CliCommand::CoreBuildBuiltin => commands::core_build_builtin::command().await?,
		cli::CliCommand::Cbor(command) => commands::cbor::command(command).await?,
	})
}
