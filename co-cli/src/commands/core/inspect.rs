// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
