use super::Command as RoomCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use cid::Cid;
use co_core_co::CoAction;
use co_core_room::Room;
use co_messaging::{
	multimedia::{ImageInfo, ThumbnailInfo},
	state_event::{RoomAvatarContent, RoomNameContent, RoomTopicContent},
	MatrixEvent,
};
use co_primitives::CoreName;
use co_sdk::{
	state::{query_core, QueryError, QueryExt},
	tags, CoDate, CoReducerFactory, Cores, CO_CORE_NAME_CO, CO_CORE_ROOM,
};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Optional name for the room
	#[arg(short, long)]
	pub name: Option<String>,

	/// Optional description for the room
	#[arg(short, long)]
	pub description: Option<String>,

	/// Optional avatar for the room in form of a CID of an image file
	#[arg(short, long)]
	pub avatar: Option<Cid>,
}

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
	match query_core(CoreName::<Room>::new(&room_command.core))
		.execute_reducer(&co_reducer)
		.await
	{
		Err(QueryError::NotFound(_)) => {
			let create = CoAction::CoreCreate {
				core: core.to_owned(),
				binary: Cores::default().binary(CO_CORE_ROOM).expect(CO_CORE_ROOM),
				tags: tags!("core": CO_CORE_ROOM),
			};
			co_reducer.push(&identity, CO_CORE_NAME_CO, &create).await?;
		},
		_ => return Err(anyhow!("Room core with ID {} already exists", core)),
	};

	let timestamp = application.context().date().now();

	// set name of new room
	let set_name = MatrixEvent::new(
		uuid::Uuid::new_v4(),
		timestamp,
		core,
		RoomNameContent::new(command.name.clone().unwrap_or("New room".to_owned())),
	);
	co_reducer.push(&identity, core, &set_name).await?;

	// set avatar of new room if given
	if let Some(avatar) = &command.avatar {
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

	if let Some(description) = &command.description {
		let set_description =
			MatrixEvent::new(uuid::Uuid::new_v4(), timestamp, core, RoomTopicContent::new(description));
		co_reducer.push(&identity, core, &set_description).await?;
	}

	Ok(exitcode::OK)
}
