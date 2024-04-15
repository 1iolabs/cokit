use super::Command as RoomCommand;
use crate::{cli::Cli, library::application::application};
use anyhow::anyhow;
use co_core_co::CoAction;
use co_messaging::{
	multimedia::{ImageInfo, ThumbnailInfo},
	state_event::{RoomAvatarContent, RoomNameContent, RoomTopicContent},
	MatrixEvent,
};
use co_sdk::{tags, CoReducerError, Cores, CO_CORE_NAME_CO, CO_CORE_ROOM};
use exitcode::ExitCode;
use libipld::Cid;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Optional name for the room
	#[arg(long)]
	pub room_name: Option<String>,

	/// Optional description for the room
	#[arg(long)]
	pub room_description: Option<String>,

	/// Optional avatar for the room in form of a CID of an image file
	#[arg(long)]
	pub avatar: Option<Cid>,
}

pub async fn command(cli: &Cli, room_command: &RoomCommand, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let identity = application.local_identity();
	let co = &room_command.co;
	let core = &room_command.core;
	let co_reducer = application.co_reducer(co).await?.ok_or(anyhow!("Co not found: {}", co))?;
	match co_reducer.state::<co_core_room::Room>(core).await {
		Err(CoReducerError::CoreNotFound(_)) => {
			let create = CoAction::CoreCreate {
				core: core.to_owned(),
				binary: Cores::default().binary(CO_CORE_ROOM).expect(CO_CORE_ROOM),
				tags: tags!("core": CO_CORE_ROOM),
			};
			co_reducer.push(&identity, CO_CORE_NAME_CO, &create).await?;
		},
		_ => return Err(anyhow!("Room core with ID {} already exists", core)),
	};

	let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

	// set name of new room
	let set_name = MatrixEvent::new(
		Cid::default(), // TODO: create unique event id
		timestamp as i64,
		core,
		RoomNameContent::new(command.room_name.clone().unwrap_or("New room".to_owned())),
	);
	co_reducer.push(&identity, core, &set_name).await?;

	// set avatar of new room if given
	if let Some(avatar) = &command.avatar {
		let set_avatar = MatrixEvent::new(
			Cid::default(), // TODO: create unique event id
			timestamp as i64,
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
		co_reducer.push(&identity, co, &set_avatar).await?;
	}

	if let Some(description) = &command.room_description {
		let set_description = MatrixEvent::new(
			Cid::default(), // TODO: create unique event id
			timestamp as i64,
			core,
			RoomTopicContent::new(description),
		);
		co_reducer.push(&identity, co, &set_description).await?;
	}

	Ok(exitcode::OK)
}
