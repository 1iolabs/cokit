// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use anyhow::{anyhow, Context};
use cid::Cid;
use co_primitives::AnyBlockStorage;
use co_storage::{unixfs_add_file, MemoryBlockStorage};
use serde::Deserialize;
use std::{
	fmt::{Debug, Display},
	fs::{canonicalize, read},
	io::ErrorKind,
	path::{Path, PathBuf},
	process::{Command, Output},
};

/// Find repositoty path by assuming the current binary is a cargo artifact.
///
/// # Args
/// - `workspace` - Go up to next workspace `Cargo.toml`
pub fn crate_repository_path(workspace: bool) -> Result<PathBuf, anyhow::Error> {
	let mut repository_path =
		PathBuf::from(option_env!("CARGO_MANIFEST_DIR").ok_or(anyhow!("Missing CARGO_MANIFEST_DIR"))?);
	if workspace {
		loop {
			match read(repository_path.join("Cargo.toml")) {
				Ok(data) => {
					let core_package: Cargo = toml::from_str(std::str::from_utf8(&data)?).context("valid toml")?;
					if core_package.workspace.is_some() {
						break;
					}
				},
				Err(err) if err.kind() == ErrorKind::NotFound => {},
				Err(err) => return Err(err.into()),
			}

			// continue to walk up
			repository_path = repository_path.parent().ok_or(anyhow!("Not found parent"))?.to_owned();
			if !repository_path.try_exists()? {
				return Err(anyhow!("Not found"));
			}
		}
	}
	// println!("env {:#?}", std::env::vars());
	// println!("exe {:?}", std::env::current_exe()?);
	// let repsotiory_path = std::env::current_exe()?
	// 	.parent()
	// 	.ok_or(anyhow!("no parent"))?
	// 	.parent()
	// 	.ok_or(anyhow!("no parent"))?
	// 	.parent()
	// 	.ok_or(anyhow!("no parent"))?
	// 	.parent()
	// 	.ok_or(anyhow!("no parent"))?
	// 	.to_owned();
	// std::fs::metadata(repsotiory_path.join("Cargo.toml"))?;
	Ok(repository_path)
}

const DEFAULT_RUSTFLAGS: &str = "-C opt-level=3 -C codegen-units=1 -C panic=abort -C strip=symbols";

/// Options for building a core to WebAssembly.
#[derive(Debug, Clone, Default)]
pub struct BuildCoreOptions {
	/// Custom RUSTFLAGS to use for the build. If `None`, uses the default flags.
	pub rustflags: Option<String>,
}

/// Build a rust core to WebAssembly using cargo.
pub fn build_core(
	repository_path: impl AsRef<Path>,
	core_path: impl AsRef<Path>,
) -> Result<BuildCoreArtifact, anyhow::Error> {
	build_core_with_options(repository_path, core_path, BuildCoreOptions::default())
}

/// Build a rust core to WebAssembly using cargo with custom options.
pub fn build_core_with_options(
	repository_path: impl AsRef<Path>,
	core_path: impl AsRef<Path>,
	options: BuildCoreOptions,
) -> Result<BuildCoreArtifact, anyhow::Error> {
	let core_path = core_path.as_ref().to_owned();
	let target_path = canonicalize(repository_path.as_ref())
		.context(format!("path: {:?}", repository_path.as_ref()))?
		.join("target-wasm");

	// read toml for name
	let core_toml = core_path.join("Cargo.toml");
	let data = read(&core_toml).context(format!("read {:?}", core_toml))?;
	let core_package: Cargo = toml::from_str(std::str::from_utf8(&data)?).context(format!("toml {:?}", core_toml))?;
	let core_package = core_package.package.ok_or(anyhow!("Missing package: {:?}", core_toml))?;

	// build
	let rustflags = options.rustflags.as_deref().unwrap_or(DEFAULT_RUSTFLAGS);
	let mut command: Command = Command::new("cargo");
	command.current_dir(&core_path).env("RUSTFLAGS", rustflags).args([
		"build",
		"--features",
		"core",
		"--target",
		"wasm32-unknown-unknown",
		"--target-dir",
		target_path.to_str().ok_or(anyhow!("Invalid path: {:?}", target_path))?,
		"--release",
		// "--message-format=json",
	]);
	let output = command.output()?;
	tracing::trace!(?output, ?command, "cargo-build");
	if !output.status.success() {
		return Err(BuildError { core_path, output }.into());
	}

	// read wasm
	let core_wasm_name = format!("{}.wasm", core_package.name.replace('-', "_"));
	let core_wasm_path = target_path.join("wasm32-unknown-unknown/release").join(&core_wasm_name);
	Ok(BuildCoreArtifact { name: core_package.name, version: core_package.version, artifact_path: core_wasm_path })
}

pub struct BuildCoreArtifact {
	pub name: String,
	pub version: String,
	pub artifact_path: PathBuf,
}
impl BuildCoreArtifact {
	/// Store the artifact to storage.
	pub async fn store_artifact(&self, storage: &impl AnyBlockStorage) -> Result<Cid, anyhow::Error> {
		unixfs_add_file(storage, &self.artifact_path).await
	}

	/// Compute Cid for the artifact.
	pub async fn cid(&self) -> Result<Cid, anyhow::Error> {
		let storage = MemoryBlockStorage::default();
		self.store_artifact(&storage).await
	}
}

#[derive(Debug, Deserialize)]
struct Cargo {
	#[serde(default)]
	package: Option<CargoPackage>,
	#[serde(default)]
	workspace: Option<CargoWorkspace>,
}

#[derive(Debug, Deserialize)]
struct CargoWorkspace {}

#[derive(Debug, Deserialize)]
struct CargoPackage {
	name: String,
	version: String,
}

#[derive(Debug, thiserror::Error)]
struct BuildError {
	core_path: PathBuf,
	output: Output,
}
impl Display for BuildError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let stdout = std::str::from_utf8(&self.output.stdout)
			.map(|s| s.to_owned())
			.unwrap_or_else(|_| format!("{:02X?}", &self.output.stdout));
		let stderr = std::str::from_utf8(&self.output.stderr)
			.map(|s| s.to_owned())
			.unwrap_or_else(|_| format!("{:02X?}", &self.output.stderr));
		let output = format!(
			"Build core failed.\nstatus: {:?}\ncore: {:?}\nstdout:\n{}\nstderr:\n{}",
			self.output.status, self.core_path, stdout, stderr
		);
		f.write_str(&output)
	}
}
