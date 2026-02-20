use super::Command as RoomCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use co_messaging::{message_event::TextContent, MatrixEvent};
use co_sdk::{CoDate, CoReducerFactory};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	pub message: String,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	room_command: &RoomCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let cli_identity = application.private_identity(&"did:local:cli".to_owned()).await?;
	let co = &room_command.co;
	let core = &room_command.core;
	let co_reducer = application.context().try_co_reducer(co).await?;
	let timestamp = application.context().date().now();
	let message_event =
		MatrixEvent::new(uuid::Uuid::new_v4(), timestamp, core, TextContent::new(command.message.clone()));
	co_reducer.push(&cli_identity, core, &message_event).await?;
	Ok(exitcode::OK)
}
