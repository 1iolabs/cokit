use crate::{cli::Cli, library::application::application};
use co_sdk::CreateCo;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: String,

	/// CO Name
	pub name: Option<String>,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;

	// create
	let create = CreateCo {
		id: command.co.clone(),
		algorithm: Some(Default::default()),
		name: command.name.as_ref().unwrap_or(&command.co).clone(),
	};
	let reducer = application.create_co(create).await?;

	// result
	println!("{} | {}", &command.co, reducer.reducer_state().await.0.expect("state"));

	// result
	Ok(exitcode::OK)
}
