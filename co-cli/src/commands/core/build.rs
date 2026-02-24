// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, commands::core::Command as CoreCommand, library::cli_context::CliContext};
use anyhow::anyhow;
#[cfg(feature = "llvm")]
use co_runtime::compile_native;
use co_sdk::build_core;
use exitcode::ExitCode;
use serde::{Deserialize, Serialize};
#[cfg(feature = "llvm")]
use std::collections::BTreeMap;
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

	/// Extra native target triple to build ahead of time.
	/// Example: `--target aarch64-apple-darwin`
	#[cfg(feature = "llvm")]
	#[arg(long)]
	pub target: Vec<String>,
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

	// result
	let mut result = JsonResult {
		name: artifact.name,
		version: artifact.version,
		artifact_path: artifact.artifact_path,
		#[cfg(feature = "llvm")]
		target_path: Default::default(),
	};

	// native target
	#[cfg(feature = "llvm")]
	for target in &command.target {
		// path
		let target_path = create_target_path(&result.artifact_path, target, "dylib")?;

		// log
		tracing::info!(?target, ?target_path, "building-core-native");

		// compile
		let wasm_bytes = tokio::fs::read(&result.artifact_path).await?;
		let target_bytes = compile_native(wasm_bytes, target).await?;

		// store
		if let Some(parent_path) = target_path.parent() {
			tokio::fs::create_dir_all(parent_path).await?;
		}
		tokio::fs::write(&target_path, &target_bytes).await?;

		// add
		result.target_path.insert(target.to_owned(), target_path);
	}

	// output
	println!("{}", serde_json::to_string_pretty(&result)?);

	// result
	Ok(exitcode::OK)
}

#[derive(Debug, Serialize)]
struct JsonResult {
	pub name: String,
	pub version: String,
	pub artifact_path: PathBuf,
	#[cfg(feature = "llvm")]
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub target_path: BTreeMap<String, PathBuf>,
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

#[cfg(any(test, feature = "llvm"))]
fn create_target_path(artifact_path: &Path, target: &str, extension: &str) -> Result<PathBuf, anyhow::Error> {
	let mut components = artifact_path.components().collect::<Vec<_>>();
	let len = components.len();
	if len >= 3 {
		let filename = PathBuf::from(components[len - 1].as_os_str()).with_extension(extension);
		components[len - 1] = std::path::Component::Normal(filename.as_os_str());
		components[len - 3] = std::path::Component::Normal(target.as_ref());
		Ok(components.into_iter().collect())
	} else {
		Err(anyhow!("Create target path failed: {}.{} path: {:?}", target, extension, artifact_path))
	}
}

#[cfg(test)]
mod tests {
	use crate::commands::core::build::create_target_path;
	use std::path::PathBuf;

	#[test]
	fn test_create_target_path() {
		let path = PathBuf::from(
			"/Users/dominik/Workspaces/test/my-todo-app/target-wasm/wasm32-unknown-unknown/release/my_todo_core.wasm",
		);
		assert_eq!(
			create_target_path(&path, "aarch64-apple-darwin", "dylib").unwrap(),
			PathBuf::from("/Users/dominik/Workspaces/test/my-todo-app/target-wasm/aarch64-apple-darwin/release/my_todo_core.dylib")
		);
	}

	#[test]
	fn test_create_target_path_short() {
		let path = PathBuf::from("wasm32-unknown-unknown/release/my_todo_core.wasm");
		assert_eq!(
			create_target_path(&path, "aarch64-apple-darwin", "dylib").unwrap(),
			PathBuf::from("aarch64-apple-darwin/release/my_todo_core.dylib")
		);
	}

	#[test]
	fn test_create_target_path_invalid() {
		let path = PathBuf::from("my_todo_core.wasm");
		assert_eq!(
			create_target_path(&path, "aarch64-apple-darwin", "dylib")
				.unwrap_err()
				.to_string(),
			r#"Create target path failed: aarch64-apple-darwin.dylib path: "my_todo_core.wasm""#
		);
	}
}
