// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	commands::{co, core, did, file, ipld, network, pin, room, schemars, storage},
	library::cli_context::CliContext,
};
use clap::{ArgAction, ValueEnum};
use exitcode::ExitCode;
use std::path::PathBuf;

const APP_IDENTIFIER: &str = "co-cli";

/// CO CLI
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,

	/// The instance ID of the daemon. Must be uniqure for every instance that runs in parallel.
	#[arg(long, default_value_t = String::from(APP_IDENTIFIER))]
	pub instance_id: String,

	/// Base path.
	///
	/// If this option ispecified all files are stored in this path (if not explicitly overwritten):
	/// - storage_path: <base_path>/storage
	/// - config_path: <base_path>/etc
	/// - log_path: <base_path>/log
	///
	/// Default: `~/Application Support/co.app.1io.co`
	#[arg(long)]
	pub base_path: Option<PathBuf>,

	/// Log path.
	#[arg(long)]
	pub log_path: Option<PathBuf>,

	/// Disable logging to file.
	#[arg(long)]
	pub no_log: bool,

	/// Only log level and above.
	#[arg(long, value_enum, default_value_t)]
	pub log_level: CliLogLevel,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	#[arg(long)]
	pub no_keychain: bool,

	/// No output.
	#[arg(short)]
	pub quiet: bool,

	/// Verbose level.
	/// By default prints info and above levels. To prevent this use `quiet` option.
	#[arg(short, default_value_t = 1, action = ArgAction::Count)]
	pub verbose: u8,

	/// Enable open telemetry tracing to endpoint.
	#[arg(long)]
	pub open_telemetry: bool,

	/// Open telemetry endpoint.
	#[arg(long, default_value_t = String::from("http://localhost:4317"))]
	pub open_telemetry_endpoint: String,

	/// Disable default features.
	#[arg(long)]
	pub no_default_features: bool,

	/// Enable feature.
	#[arg(long, short = 'F')]
	pub feature: Vec<String>,
}

#[derive(Debug, Default, Clone, ValueEnum)]
pub enum CliLogLevel {
	Error,
	Warn,
	#[default]
	Info,
	Debug,
	Trace,
}
impl CliLogLevel {
	pub fn to_level(&self) -> tracing::Level {
		match self {
			CliLogLevel::Error => tracing::Level::ERROR,
			CliLogLevel::Warn => tracing::Level::WARN,
			CliLogLevel::Info => tracing::Level::INFO,
			CliLogLevel::Debug => tracing::Level::DEBUG,
			CliLogLevel::Trace => tracing::Level::TRACE,
		}
	}
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
	/// CO.
	Co(co::Command),

	/// Network Utilities.
	Network(network::Command),

	/// COre related commands.
	Core(core::Command),

	/// IPLD Utilities.
	Ipld(ipld::Command),

	/// Identities
	Did(did::Command),

	/// Block Storage.
	Storage(storage::Command),

	/// File.
	File(file::Command),

	/// Room
	Room(room::Command),

	/// Pin
	Pin(pin::Command),

	/// Json schemas
	Schemars(schemars::Command),
}

#[tracing::instrument(level = tracing::Level::INFO, err(Debug), ret, skip(cli))]
pub async fn command(cli: &Cli) -> Result<ExitCode, anyhow::Error> {
	// trace arguments
	tracing::debug!(?cli, "arguments");

	// context
	let context = CliContext::default();

	// execute
	let result = match &cli.command {
		CliCommand::Co(command) => co::command(&context, cli, command).await,
		CliCommand::Network(command) => network::command(&context, cli, command).await,
		CliCommand::Core(command) => core::command(&context, cli, command).await,
		CliCommand::Ipld(command) => ipld::command(&context, command).await,
		CliCommand::Did(command) => did::command(&context, cli, command).await,
		CliCommand::Storage(command) => storage::command(&context, cli, command).await,
		CliCommand::File(command) => file::command(&context, cli, command).await,
		CliCommand::Room(command) => room::command(&context, cli, command).await,
		CliCommand::Pin(command) => pin::command(&context, cli, command).await,
		CliCommand::Schemars(command) => schemars::command(&context, cli, command).await,
	};

	// shutdown and wait for tasks to complete
	context.tasks.close();
	context.tasks.wait().await;

	// result
	result
}
