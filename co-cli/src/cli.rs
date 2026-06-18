// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	commands::{co, core, did, file, ipld, network, room, schemars, storage},
	library::cli_context::CliContext,
};
use co_tracing::LogArgs;
use exitcode::ExitCode;
use std::path::PathBuf;

const APP_IDENTIFIER: &str = "co-cli";

/// CO CLI
#[derive(Debug, Clone, clap::Parser)]
#[command(version)]
pub struct Cli {
	/// Command.
	#[command(subcommand)]
	pub command: CliCommand,

	/// The instance ID of the daemon. Must be unique for every instance that runs in parallel.
	#[arg(long, default_value_t = String::from(APP_IDENTIFIER), env = "CO_INSTANCE_ID")]
	pub instance_id: String,

	/// Base path.
	///
	/// If this option is specified all files are stored in this path (if not explicitly overwritten):
	/// - storage_path: <base_path>/storage
	/// - config_path: <base_path>/etc
	/// - log_path: <base_path>/log
	///
	/// Default: `~/Application Support/co.app.1io.co`
	#[arg(long, env = "CO_BASE_PATH")]
	pub base_path: Option<PathBuf>,

	/// Logging configuration (`CO_LOG`, `-v`/`-q`, OpenTelemetry — see `co-tracing`).
	#[command(flatten)]
	pub log: LogArgs,

	/// Read/Write Local CO encryption key to file instead of the OS keychain.
	///
	/// Warning: This option is INSECURE only use when you know the implications.
	#[arg(long, default_value_t = false, env = "CO_NO_KEYCHAIN", value_parser = parse_bool)]
	pub no_keychain: bool,

	/// Disable default features.
	#[arg(long)]
	pub no_default_features: bool,

	/// Enable feature.
	#[arg(long, short = 'F')]
	pub feature: Vec<String>,
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
		CliCommand::Schemars(command) => schemars::command(&context, cli, command).await,
	};

	// shutdown and wait for tasks to complete
	context.tasks.close();
	context.tasks.wait().await;

	// result
	result
}

fn parse_bool(s: &str) -> Result<bool, String> {
	match s {
		"1" | "true" => Ok(true),
		"0" | "false" => Ok(false),
		_ => Err(format!("invalid bool: {s}")),
	}
}
