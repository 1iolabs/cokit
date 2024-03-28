use clap::Parser;
use library::application::log_path;
use tracing::Level;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};

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

	// tracing: verbose
	let output = if !cli.quiet {
		let writer = match cli.verbose {
			0 => std::io::stderr.with_max_level(Level::WARN),
			1 => std::io::stderr.with_max_level(Level::INFO),
			2 => std::io::stderr.with_max_level(Level::DEBUG),
			_ => std::io::stderr.with_max_level(Level::TRACE),
		};
		Some(tracing_subscriber::fmt::layer().with_writer(writer))
	} else {
		None
	};

	// tracing: log
	let log = if !cli.no_log {
		let log_path = log_path(&cli);
		tokio::fs::create_dir_all(log_path.parent().ok_or(anyhow::anyhow!("no parent"))?).await?;
		let log_file = std::fs::File::create(log_path)?;
		let formatting_layer =
			BunyanFormattingLayer::new(cli.instance_id.to_owned(), log_file.with_max_level(Level::TRACE));
		Some(formatting_layer)
	} else {
		None
	};

	// tracing
	tracing_subscriber::registry()
		.with(JsonStorageLayer)
		.with(output)
		.with(log)
		.init();

	// execute
	cli::command(&cli).await
}
