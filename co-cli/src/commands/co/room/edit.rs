use super::{create::Command, Command as RoomCommand};
use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use co_messaging::{state_event::RoomNameContent, MatrixEvent};
use exitcode::ExitCode;
use libipld::Cid;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn command(cli: &Cli, room_command: &RoomCommand, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let identity = application.local_identity();
	let co = &room_command.co_id;
	let core = &room_command.room_id;
	let co_reducer = application.co_reducer(co).await?.ok_or(anyhow!("Co not found: {}", co))?;

	let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

	if let Some(name) = &command.room_name {
		// set new name of room
		let set_name = MatrixEvent::new(
			Cid::default(), // TODO: create unique event id
			timestamp.into(),
			core,
			RoomNameContent::new(name),
		);
		co_reducer.push(&identity, core, &set_name).await?;
	}

	Ok(exitcode::OK)
}
