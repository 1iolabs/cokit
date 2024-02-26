use crate::{cli::Cli, library::application::application};
use co_sdk::{CoId, CreateCo};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// CO Name
	pub name: Option<String>,

	/// Public (unencrypted)
	#[arg(short, default_value_t = false)]
	pub public: bool,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;

	// create
	let create = CreateCo {
		id: command.co.clone(),
		algorithm: if command.public { None } else { Some(Default::default()) },
		name: command.name.as_deref().unwrap_or(command.co.as_str()).to_string(),
	};
	let reducer = application.create_co(create).await?;

	// result
	println!("{} | {}", &command.co, reducer.reducer_state().await.0.expect("state"));

	// result
	Ok(exitcode::OK)
}
