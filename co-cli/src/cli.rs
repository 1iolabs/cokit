use crate::{
	commands::{co, core_build_builtin, did, file, ipld, network, pin, room, schemars, storage},
	library::cli_context::CliContext,
};
use clap::ArgAction;
use exitcode::ExitCode;
use std::path::PathBuf;
use tracing::instrument;

const APP_IDENTIFIER: &str = "co-cli";

/// Run COs via an HTTP Daemon.
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
	#[arg(long, default_value_t = false)]
	pub no_log: bool,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	#[arg(long, default_value_t = false)]
	pub no_keychain: bool,

	/// No output.
	#[arg(short, default_value_t = false)]
	pub quiet: bool,

	/// Verbose level.
	/// By default prints info and above levels. To precent this use `quiet` option.
	#[arg(short, default_value_t = 1, action = ArgAction::Count)]
	pub verbose: u8,

	/// Enable open telemetry tracing to endpoint.
	#[arg(long)]
	pub open_telemetry: bool,

	/// Open telemetry endpoint.
	#[arg(long, default_value_t = String::from("http://localhost:4317"))]
	pub open_telemetry_endpoint: String,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
	/// CO.
	Co(co::Command),

	/// Network Utilities.
	Network(network::Command),

	/// Build the build-in cores.
	CoreBuildBuiltin,

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

#[instrument(err, ret, skip(cli), level = "debug")]
pub async fn command(cli: &Cli) -> Result<ExitCode, anyhow::Error> {
	// trace arguments
	tracing::debug!(?cli, "arguments");

	// context
	let context = CliContext::default();

	// execute
	let result = match &cli.command {
		CliCommand::Co(command) => co::command(&context, cli, command).await,
		CliCommand::Network(command) => network::command(&context, cli, command).await,
		CliCommand::CoreBuildBuiltin => core_build_builtin::command().await,
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
