use anyhow::{anyhow, Context};
use cid::Cid;
use co_storage::{unixfs_add_file, BlockStorage, MemoryBlockStorage};
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
	while workspace {
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

/// Build a rust core to WebAssembly using cargo.
pub fn build_core(
	repository_path: impl AsRef<Path>,
	core_path: impl AsRef<Path>,
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
	let mut command: Command = Command::new("cargo");
	command
		.current_dir(&core_path)
		.env("RUSTFLAGS", "-C opt-level=z -C codegen-units=1 -C panic=abort -C strip=symbols")
		.args([
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

	// let output = std::str::from_utf8(&output.stdout).unwrap();
	// for line in output.lines() {
	// 	let json: serde_json::Value = serde_json::from_str(&line)?;
	// 	println!("json: {:#?}", json);
	// }
	// let output: BuildOutput = serde_json::from_slice(&output.stdout)?;
	// Ok(output.filenames.first().ok_or(anyhow!("No artifacts"))?.clone())
}

pub struct BuildCoreArtifact {
	pub name: String,
	pub version: String,
	pub artifact_path: PathBuf,
}
impl BuildCoreArtifact {
	/// Store the artifact to storage.
	pub async fn store_artifact<S>(&self, storage: &S) -> Result<Cid, anyhow::Error>
	where
		S: BlockStorage + 'static,
	{
		Ok(unixfs_add_file(storage, &self.artifact_path).await?)
	}

	/// Compute Cid for the artifact.
	pub async fn cid<S>(&self) -> Result<Cid, anyhow::Error>
	where
		S: BlockStorage + 'static,
	{
		let storage = MemoryBlockStorage::default();
		Ok(self.store_artifact(&storage).await?)
	}
}

// // {"reason":"compiler-artifact","package_id":"path+file:///Users/dominik/Workspaces/test/co/examples/counter#example-counter@0.1.0","manifest_path":"/Users/dominik/Workspaces/test/co/examples/counter/Cargo.toml","target":{"kind":["lib","cdylib"],"crate_types":["lib","cdylib"],"name":"example_counter","src_path":"/Users/dominik/Workspaces/test/co/examples/counter/src/lib.rs","edition":"2021","doc":true,"doctest":true,"test":true},"profile":{"opt_level":"3","debuginfo":0,"debug_assertions":false,"overflow_checks":false,"test":false},"features":[],"filenames":["/Users/dominik/Workspaces/test/co/co-sdk/../target-wasm/wasm32-unknown-unknown/release/libexample_counter.rlib","/Users/dominik/Workspaces/test/co/co-sdk/../target-wasm/wasm32-unknown-unknown/release/example_counter.wasm"],"executable":null,"fresh":true}
// #[derive(Debug, Deserialize)]
// struct BuildOutput {
// 	reason: String,
// 	manifest_path: String,
// 	#[serde(default)]
// 	filenames: Vec<PathBuf>,
// }

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
