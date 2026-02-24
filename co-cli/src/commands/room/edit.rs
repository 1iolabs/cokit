// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{create::Command, Command as RoomCommand};
use crate::{cli::Cli, library::cli_context::CliContext};
use co_messaging::{
	multimedia::{ImageInfo, ThumbnailInfo},
	state_event::{RoomAvatarContent, RoomNameContent, RoomTopicContent},
	MatrixEvent,
};
use co_sdk::CoReducerFactory;
use exitcode::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	room_command: &RoomCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let identity = application.local_identity();
	let co = &room_command.co;
	let core = &room_command.core;
	let co_reducer = application.context().try_co_reducer(co).await?;

	let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

	if let Some(name) = &command.name {
		// set new name of room
		let set_name = MatrixEvent::new(uuid::Uuid::new_v4(), timestamp, core, RoomNameContent::new(name));
		co_reducer.push(&identity, core, &set_name).await?;
	}

	if let Some(desc) = &command.description {
		// set new room description
		let set_desc = MatrixEvent::new(uuid::Uuid::new_v4(), timestamp, core, RoomTopicContent::new(desc));
		co_reducer.push(&identity, core, &set_desc).await?;
	}

	if let Some(avatar) = &command.avatar {
		// set room avatar
		let set_avatar = MatrixEvent::new(
			uuid::Uuid::new_v4(),
			timestamp,
			core,
			RoomAvatarContent::new(
				Some(*avatar),
				// TODO: generate metadata for image
				ImageInfo {
					h: 0,
					w: 0,
					mimetype: "".into(),
					size: 0,
					thumbnail_file: Default::default(),
					thumbnail_info: ThumbnailInfo { h: 0, w: 0, mimetype: "".into(), size: 0 },
				},
			),
		);
		co_reducer.push(&identity, core, &set_avatar).await?;
	}

	Ok(exitcode::OK)
}
