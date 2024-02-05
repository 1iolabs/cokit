use clap::Parser;
use co_sdk::unixfs_encode_buffer;
use libipld::DefaultParams;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env::current_exe, process::Command};

mod cli;

#[tokio::main]
async fn main() {
	let cli = cli::Cli::parse();
	match cli.command {
		cli::CliCommand::CoresBuild => {
			let paths = ["co", "keystore", "membership", "room"];

			// get repository root path
			// /Users/dominik/Workspaces/test/co/target/debug/co-cli
			let respository_path = current_exe()
				.expect("current_exe")
				.parent()
				.unwrap()
				.parent()
				.unwrap()
				.parent()
				.unwrap()
				.to_owned();

			// build cores
			for path in paths {
				let core_path = respository_path.join("cores").join(path);
				println!("build: {:?}", core_path);
				Command::new("cargo")
					.current_dir(respository_path.join("cores").join(path))
					.args(["build", "--target=wasm32-unknown-unknown", "--release"])
					.output()
					.unwrap();
			}

			// create Cids
			let mut cores: Cores = Default::default();
			for path in paths {
				let core_path = respository_path.join("cores").join(path);

				// read toml for name
				let core_toml = core_path.join("Cargo.toml");
				let data = tokio::fs::read(core_toml).await.expect("to read Cargo.toml");
				let core_package: Cargo = toml::from_str(std::str::from_utf8(&data).unwrap()).expect("valid toml");

				// read wasm
				let core_wasm_name = format!("{}.wasm", core_package.package.name.replace("-", "_"));
				let core_wasm_path = respository_path
					.join("target/wasm32-unknown-unknown/release")
					.join(core_wasm_name);
				let core_wasm = tokio::fs::read(core_wasm_path).await.expect("wasm artifact to exist");
				let core_blocks = unixfs_encode_buffer::<DefaultParams>(&core_wasm);
				let core_cid = core_blocks.last().expect("at least one block").cid().clone();

				// add
				cores.cores.insert(core_package.package.name, core_cid.to_string());
			}

			// write
			let cores_path = respository_path.join("cores/Cores.toml");
			println!("write: {:?}", cores_path);
			tokio::fs::write(cores_path, toml::to_string(&cores).unwrap().as_bytes())
				.await
				.unwrap();
		},
	}
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
	cores: HashMap<String, String>,
}
