use anyhow::{anyhow, Context};
use co_sdk::unixfs_encode_buffer;
use exitcode::ExitCode;
use libipld::DefaultParams;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env::current_exe, process::Command, str::from_utf8};

pub async fn command() -> Result<ExitCode, anyhow::Error> {
	let paths = ["co", "keystore", "membership", "room", "pin"];

	// get repository root path
	//  `<respository_path>/target/debug/co-cli`
	let respository_path = current_exe()?
		.parent()
		.ok_or(anyhow!("no parent"))?
		.parent()
		.ok_or(anyhow!("no parent"))?
		.parent()
		.ok_or(anyhow!("no parent"))?
		.to_owned();

	// build cores
	for path in paths {
		let core_path = respository_path.join("cores").join(path);
		println!("build: {:?}", core_path);
		let command = Command::new("cargo")
			.current_dir(respository_path.join("cores").join(path))
			.args(["build", "--target=wasm32-unknown-unknown", "--release"])
			.output()?;
		if !command.status.success() {
			println!("failed ({}):", command.status);
			println!("{}", from_utf8(&command.stdout).unwrap());
			println!("{}", from_utf8(&command.stderr).unwrap());
		}
	}

	// create Cids
	let mut cores: Cores = Default::default();
	for path in paths {
		let core_path = respository_path.join("cores").join(path);

		// read toml for name
		let core_toml = core_path.join("Cargo.toml");
		let data = tokio::fs::read(core_toml).await.context("to read Cargo.toml")?;
		let core_package: Cargo = toml::from_str(std::str::from_utf8(&data)?).context("valid toml")?;

		// read wasm
		let core_wasm_name = format!("{}.wasm", core_package.package.name.replace("-", "_"));
		let core_wasm_path = respository_path
			.join("target/wasm32-unknown-unknown/release")
			.join(core_wasm_name);
		let core_wasm = tokio::fs::read(core_wasm_path).await.expect("wasm artifact to exist");
		let core_blocks = unixfs_encode_buffer::<DefaultParams>(&core_wasm);
		let core_cid = core_blocks.last().ok_or(anyhow!("at least one block"))?.cid().clone();

		// add
		cores.cores.insert(core_package.package.name, core_cid.to_string());
	}

	// write
	let cores_path = respository_path.join("cores/Cores.toml");
	println!("write: {:?}", cores_path);
	tokio::fs::write(cores_path, toml::to_string(&cores)?.as_bytes()).await?;

	Ok(exitcode::OK)
}

#[derive(Debug, Serialize, Deserialize)]
struct Cargo {
	package: CargoPackage,
}

#[derive(Debug, Serialize, Deserialize)]
struct CargoPackage {
	name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cores {
	cores: BTreeMap<String, String>,
}
