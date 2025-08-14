use crate::{cli::Cli, commands::core::Command as CoreCommand, library::cli_context::CliContext};
use anyhow::anyhow;
use co_sdk::build_core;
use exitcode::ExitCode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The project path of the Core.
	///
	/// The folder where its Cargo.toml is located.
	pub core_path: Option<PathBuf>,

	/// The workspace path of the Core.
	///
	/// The folder where the workspace Cargo.toml is located.
	pub workspace_path: Option<PathBuf>,
}

pub async fn command(
	_context: &CliContext,
	_cli: &Cli,
	_core_command: &CoreCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let core_path = match &command.core_path {
		Some(path) => path.clone(),
		None => std::env::current_dir()?,
	};
	let workspace_path = match &command.workspace_path {
		Some(path) => path.clone(),
		None => find_workspace_path(&core_path)?,
	};

	// build
	tracing::info!(?workspace_path, ?core_path, "building-core");
	let artifact = build_core(workspace_path, core_path)?;

	// output
	let result = JsonResult { name: artifact.name, version: artifact.version, artifact_path: artifact.artifact_path };
	println!("{}", serde_json::to_string_pretty(&result)?);

	// result
	Ok(exitcode::OK)
}

#[derive(Debug, Serialize)]
struct JsonResult {
	pub name: String,
	pub version: String,
	pub artifact_path: PathBuf,
}

#[tracing::instrument(err(Debug), ret)]
fn find_workspace_path(core_path: &Path) -> Result<PathBuf, anyhow::Error> {
	let output = std::process::Command::new("cargo")
		.current_dir(core_path)
		.args(["metadata", "--no-deps", "--format-version=1"])
		.output()?;
	let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)?;
	let workspace_or_project_root = match metadata.workspace_root {
		Some(workspace_root) => PathBuf::from(workspace_root),
		None => core_path.to_owned(),
	};
	if !std::fs::metadata(&workspace_or_project_root)?.is_dir() {
		return Err(anyhow!("Not a directory: {:?}", workspace_or_project_root));
	}
	Ok(workspace_or_project_root)
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
	#[serde(default)]
	workspace_root: Option<String>,
}
