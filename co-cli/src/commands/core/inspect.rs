// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, commands::core::Command as CoreCommand, library::cli_context::CliContext};
use co_runtime::ModuleDescription;
use exitcode::ExitCode;
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The WASM binary to inspect.
	pub wasm_path: PathBuf,
}

pub async fn command(
	_context: &CliContext,
	_cli: &Cli,
	_core_command: &CoreCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let descriptor = ModuleDescription::from_path(&command.wasm_path).await?;

	// result
	println!("Exports:");
	for export in &descriptor.exports {
		println!("- `{}`: {}", export.0, export.1);
	}
	println!("Imports:");
	for import in &descriptor.imports {
		println!("- `{}::{}`: {}", import.0, import.1, import.2);
	}

	// result
	Ok(exitcode::OK)
}
