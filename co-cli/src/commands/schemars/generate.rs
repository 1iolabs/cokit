use super::external::cid::generate_cid_schema;
use co_core_room::Room;
use co_messaging::MatrixEvent;
use exitcode::ExitCode;
use schemars::schema::RootSchema;
use std::{fs::File, io::Write};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Module {
	Cid,
	Messaging,
	Room,
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
	for module in command.clone().modules {
		match module {
			Module::Cid => {
				let mut file = File::create(command.output.clone())?;
				file.write_all(generate_cid_schema().as_bytes())?;
			},
			Module::Messaging => {
				let schema = schemars::schema_for!(MatrixEvent);
				write_schema_file(schema, command.output.clone() + "/matrix-event.json")?;
			},
			Module::Room => {
				let schema = schemars::schema_for!(Room);
				write_schema_file(schema, command.output.clone() + "/room.json")?;
			},
		};
	}
	Ok(exitcode::OK)
}

fn write_schema_file(schema: RootSchema, path: String) -> Result<(), anyhow::Error> {
	let mut file = File::create(path)?;
	file.write_all(serde_json::to_string_pretty(&schema)?.as_bytes())?;
	Ok(())
}
