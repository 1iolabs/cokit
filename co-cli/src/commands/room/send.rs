// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::Command as RoomCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use co_messaging::{message_event::TextContent, MatrixEvent};
use co_sdk::CoReducerFactory;
use exitcode::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

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
	let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
	let message_event =
		MatrixEvent::new(uuid::Uuid::new_v4(), timestamp, core, TextContent::new(command.message.clone()));
	co_reducer.push(&cli_identity, core, &message_event).await?;
	Ok(exitcode::OK)
}
