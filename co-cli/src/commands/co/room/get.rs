use super::Command as RoomCommand;
use crate::{cli::Cli, library::application::application};
use co_messaging::MatrixEvent;
use co_primitives::ReducerAction;
use co_sdk::{BlockStorage, BlockStorageExt, MultiCodec};
use exitcode::ExitCode;
use futures::{pin_mut, StreamExt};
use serde::de::IgnoredAny;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Messages to print.
	#[arg(short, long, default_value_t = 10)]
	count: usize,
	/// Messages to skip.
	#[arg(short, long, default_value_t = 0)]
	skip: usize,
}

pub async fn command(cli: &Cli, room_command: &RoomCommand, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let (storage, stream, _mapping) = application.co_log_entries(&room_command.co_id).await?;

	// stream
	let mut index = 0;
	let stream = stream.take(command.count).skip(command.skip);
	pin_mut!(stream);
	while let Some(entry) = stream.next().await {
		match entry {
			Ok(entry) => {
				// payload
				let cid = entry.entry().payload;
				let block = storage.clone().get(&cid).await?;
				let codec = MultiCodec::from(block.cid().codec());
				match codec {
					MultiCodec::DagCbor => {
						let action: ReducerAction<IgnoredAny> = storage.get_deserialized(&cid).await?;
						// skip actions from other cores
						if action.core != room_command.room_id {
							continue;
						}
						let event: ReducerAction<MatrixEvent> = storage.get_deserialized(&cid).await?;
						print_message(event);
					},
					_ => (),
				}
				println!("");
			},
			Err(err) => println!("head ({index}) error: {:?}", err),
		}
		index += 1;
	}

	// result
	Ok(exitcode::OK)
}

fn print_message(action: ReducerAction<MatrixEvent>) {
	let event = action.payload;
	match event.content {
		co_messaging::EventContent::Message(message) => match message {
			co_messaging::message_event::MessageType::Text(content) =>
				println!("{:?}: {:?}", action.from, content.body),
			_ => (),
		},
		co_messaging::EventContent::State(state) => match state {
			co_messaging::state_event::StateType::RoomName(name) =>
				println!("{:?} changed the room name to: {:?}", action.from, name.name),
			_ => (),
		},
		_ => (),
	}
}
