use super::Command as RoomCommand;
use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use co_messaging::{message_event::TextContent, MatrixEvent};
use exitcode::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	pub message: String,
}

pub async fn command(cli: &Cli, room_command: &RoomCommand, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let identity = application.local_identity();
	let co = &room_command.co_id;
	let core = &room_command.room_id;
	let co_reducer = application.co_reducer(co).await?.ok_or(anyhow!("Co not found: {}", co))?;
	let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
	let message_event = MatrixEvent::new(
		"some_event", // TODO unique event id
		timestamp,
		core,
		TextContent::new(command.message.clone()),
	);
	co_reducer.push(&identity, core, &message_event).await?;
	Ok(exitcode::OK)
}
