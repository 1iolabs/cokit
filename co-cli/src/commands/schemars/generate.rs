use co_core_keystore::Key;
use co_core_room::Room;
use co_messaging::MatrixEvent;
use co_sdk::Cores;
use exitcode::ExitCode;
use schemars::schema::RootSchema;
use std::{fs::File, io::Write};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Module {
	Messaging,
	Room,
	Key,
	Cores,
}

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// A list of the modules you want to generate the schemas for
	#[arg(short, long, value_delimiter = ' ', num_args = 1..)]
	pub modules: Vec<Module>,
	/// Optional alternate output path
	#[arg(short, long, default_value = "./json-schemas")]
	pub output: String,
}

pub async fn command(command: &Command) -> Result<ExitCode, anyhow::Error> {
	let mut index_ts = "".to_owned();
	for module in command.clone().modules {
		match module {
			Module::Messaging => {
				let schema = schemars::schema_for!(MatrixEvent);
				write_schema_file(schema, command.output.clone() + "/schemas/matrix-event.json")?;
				index_ts = format!("{}export * as Messaging from \"./matrix-event.js\"\n", index_ts);
			},
			Module::Room => {
				let schema = schemars::schema_for!(Room);
				write_schema_file(schema, command.output.clone() + "/schemas/room.json")?;
				index_ts = format!("{}export * as Room from \"./room.js\"\n", index_ts);
			},
			Module::Cores => {
				let mut ts_enum: String = "export enum Cores {\n".to_owned();
				let built_in_cores = Cores::default().built_in();
				for (core_id, core) in built_in_cores {
					let core_cid = match core {
						co_runtime::Core::Wasm(cid) => cid,
						_ => continue,
					};
					ts_enum = format!("{}\t\"{}\" = \"{}\",\n", ts_enum, core_id, core_cid);
				}
				ts_enum = format!("{}}}", ts_enum);
				let mut file = File::create(command.output.clone() + "/types/cores.ts")?;
				file.write_all(ts_enum.as_bytes())?;
				index_ts = format!("{}export * as Cores from \"./cores.js\"\n", index_ts);
			},
			Module::Key => {
				let schema = schemars::schema_for!(Key);
				write_schema_file(schema, command.output.clone() + "/schemas/keystore-key.json")?;
				index_ts = format!("{}export * as Keystore from \"./keystore-key.js\"\n", index_ts);
			},
		};
		let mut file = File::create(command.output.clone() + "/types/index.ts")?;
		file.write_all(index_ts.as_bytes())?;
	}
	Ok(exitcode::OK)
}

fn write_schema_file(schema: RootSchema, path: String) -> Result<(), anyhow::Error> {
	let mut file = File::create(path)?;
	file.write_all(serde_json::to_string_pretty(&schema)?.as_bytes())?;
	Ok(())
}
