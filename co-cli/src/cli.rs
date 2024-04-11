use crate::commands::{cbor, co, core_build_builtin, file, pin, room};
use clap::ArgAction;
use exitcode::ExitCode;
use std::path::PathBuf;

pub const APP_IDENTIFIER: &str = "co-cli";

/// Run COs via an HTTP Daemon.
#[derive(Debug, Clone, clap::Parser)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,

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
	#[arg(short, default_value_t = 1, action = ArgAction::Count)]
	pub verbose: u8,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
	/// DAG-CBOR Utilities.
	Co(co::Command),

	/// Build the build-in cores.
	CoreBuildBuiltin,

	/// DAG-CBOR Utilities.
	Cbor(cbor::Command),

	/// File.
	File(file::Command),

	/// Room
	Room(room::Command),

	/// Pin
	Pin(pin::Command),
}

pub async fn command(cli: &Cli) -> Result<ExitCode, anyhow::Error> {
	match &cli.command {
		CliCommand::Co(command) => co::command(&cli, &command).await,
		CliCommand::CoreBuildBuiltin => core_build_builtin::command().await,
		CliCommand::Cbor(command) => cbor::command(command).await,
		CliCommand::File(command) => file::command(cli, command).await,
		CliCommand::Room(command) => room::command(cli, command).await,
		CliCommand::Pin(command) => pin::command(cli, command).await,
	}
}
