use super::Command as RoomCommand;
use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	pub message: String,
}

pub async fn command(cli: &Cli, room_command: &RoomCommand, _command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let _identity = application.local_identity();
	let co = &room_command.co_id;
	let _core = &room_command.room_id;
	let _co_reducer = application.co_reducer(co).await?.ok_or(anyhow!("Co not found: {}", co))?;
	Ok(exitcode::OK)
}
