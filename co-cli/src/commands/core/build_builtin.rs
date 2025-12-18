use anyhow::anyhow;
use co_primitives::DefaultParams;
use co_sdk::{build_core, crate_repository_path, unixfs_encode_buffer};
use exitcode::ExitCode;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, future::ready, os::unix::ffi::OsStrExt, path::PathBuf};
use tokio_stream::wrappers::ReadDirStream;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Only compile specific cores (folder name).
	pub core: Vec<String>,
}

pub async fn command(command: &Command) -> Result<ExitCode, anyhow::Error> {
	// get repository root path
	let repository_path = crate_repository_path(true).unwrap();

	// paths
	// let paths = ["co", "keystore", "membership", "room", "pin", "file"];
	let paths: Vec<PathBuf> = ReadDirStream::new(tokio::fs::read_dir(repository_path.join("cores")).await?)
		.filter_map(|entry| async move {
			match entry {
				Ok(entry) => match entry.file_type().await {
					Ok(file_type) if file_type.is_dir() => Some(Ok(PathBuf::from(entry.file_name()))),
					Err(e) => Some(Err(e)),
					_ => None,
				},
				Err(e) => Some(Err(e)),
			}
		})
		.try_filter(|entry| {
			ready({
				if command.core.is_empty() {
					true
				} else if let Some(name) = entry.file_name().and_then(|name| std::str::from_utf8(name.as_bytes()).ok())
				{
					command.core.iter().find(|filter| filter.as_str() == name).is_some()
				} else {
					true
				}
			})
		})
		.try_collect()
		.await?;

	// build cores
	let build_artifacts = paths
		.iter()
		.map(|path| {
			let core_path = repository_path.join("cores").join(path);
			println!("build: {:?}", core_path);
			build_core(&repository_path, core_path)
		})
		.collect::<Result<Vec<_>, _>>()?;

	// create Cids
	let mut cores: Cores = Default::default();
	for build_artifact in build_artifacts {
		let core_wasm = tokio::fs::read(&build_artifact.artifact_path)
			.await
			.expect("wasm artifact to exist");
		let core_blocks = unixfs_encode_buffer::<DefaultParams>(&core_wasm);
		let core_cid = *core_blocks
			.last()
			.ok_or(anyhow!("{:?} to be at least one block", build_artifact.artifact_path))?
			.cid();

		// add
		cores.cores.insert(build_artifact.name, core_cid.to_string());
	}

	// write
	let cores_path = repository_path.join("cores/Cores.toml");
	println!("write: {:?}", cores_path);
	tokio::fs::write(cores_path, toml::to_string(&cores)?.as_bytes()).await?;

	Ok(exitcode::OK)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cores {
	cores: BTreeMap<String, String>,
}
