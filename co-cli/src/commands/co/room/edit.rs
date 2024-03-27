use super::{create::Command, Command as RoomCommand};
use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use co_messaging::{
	multimedia::{ImageInfo, ThumbnailInfo},
	state_event::{RoomAvatarContent, RoomNameContent, RoomTopicContent},
	MatrixEvent,
};
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
			"event_id", // TODO: create unique event id
			timestamp.into(),
			core,
			RoomNameContent::new(name),
		);
		co_reducer.push(&identity, core, &set_name).await?;
	}

	if let Some(desc) = &command.room_description {
		// set new room description
		let set_desc = MatrixEvent::new(
			"event_id", // TODO create unique ID
			timestamp.into(),
			core,
			RoomTopicContent::new(desc),
		);
		co_reducer.push(&identity, &core, &set_desc).await?;
	}

	if let Some(avatar) = &command.avatar {
		// set room avatar
		let set_avatar = MatrixEvent::new(
			"event_id", // TODO create unique ID
			timestamp.into(),
			core,
			RoomAvatarContent::new(
				*avatar,
				// TODO: generate metadata for image
				ImageInfo {
					h: 0,
					w: 0,
					mimetype: "".into(),
					size: 0,
					thumbnail_file: Cid::default(),
					thumbnail_info: ThumbnailInfo { h: 0, w: 0, mimetype: "".into(), size: 0 },
				},
			),
		);
		co_reducer.push(&identity, &core, &set_avatar).await?;
	}

	Ok(exitcode::OK)
}
