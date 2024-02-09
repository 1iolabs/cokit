use crate::{cli::Cli, library::application::application};
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
	let reducer = application
		.create_co(&command.co, command.name.as_ref().unwrap_or(&command.co))
		.await?;

	// result
	println!("{} | {}", &command.co, reducer.reducer_state().await.0.expect("state"));

	// result
	Ok(exitcode::OK)
}
