use super::Command as RoomCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use chrono::{DateTime, Local};
use co_core_room::Room;
use co_messaging::MatrixEvent;
use co_primitives::ReducerAction;
use co_sdk::{BlockStorageExt, CoReducerFactory};
use exitcode::ExitCode;
use futures::pin_mut;
use serde::de::IgnoredAny;
use std::{
	time::{Duration, UNIX_EPOCH},
	usize,
};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Events to print.
	#[arg(short, long, default_value_t = 10)]
	count: usize,
	/// Events to skip.
	#[arg(short, long, default_value_t = 0)]
	skip: usize,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	room_command: &RoomCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let co_reducer = application.context().try_co_reducer(&room_command.co).await?;

	let state = co_reducer.state::<Room>(&room_command.core).await?;
	let (storage, stream, _mapping) = application.co().entries(&room_command.co).await?;

	// print header
	println!("History of room '{}' (core: {})", state.name, room_command.core);
	println!("Printing {} events from earliest to latest after skipping {} events", command.count, command.skip);
	// terminal wide hline
	let (x, _y) = termion::terminal_size().unwrap();
	println!("{:=<width$}", "=", width = x as usize);

	// stream
	let mut index = 0;
	let stream = stream.skip(command.skip).take(command.count);
	pin_mut!(stream);
	while let Some(entry) = stream.next().await {
		match entry {
			Ok(entry) => {
				// payload
				let cid = entry.entry().payload;
				// resolve reducer action header only
				let action: ReducerAction<IgnoredAny> = storage.get_deserialized(&cid).await?;
				// skip actions from other cores
				if action.core != room_command.core {
					continue;
				}
				// resolve complete reducer action
				let action: ReducerAction<MatrixEvent> = storage.get_deserialized(&cid).await?;
				print_message(action);
			},
			Err(err) => println!("head ({index}) error: {:?}", err),
		}
		index += 1;
	}

	// result
	Ok(exitcode::OK)
}

fn print_message(action: ReducerAction<MatrixEvent>) {
	// TODO move everything timestamp related to primitives
	// calc system time from unix ts
	let d = UNIX_EPOCH + Duration::from_millis(action.time.try_into().unwrap());
	let datetime = DateTime::<Local>::from(d);
	// format a datetime string
	let timestamp_str = datetime.format("%d.%m.%Y, %H:%M:%S").to_string();

	let event = action.payload;
	match event.content {
		co_messaging::EventContent::Message(message) => match message {
			co_messaging::message_event::MessageType::Text(content) => {
				println!("{} ({}): {}", action.from, timestamp_str, content.body)
			},
			_ => (),
		},
		co_messaging::EventContent::State(state) => match state {
			co_messaging::state_event::StateType::RoomName(name) => {
				println!("{} changed the room name to: '{}' ({})", action.from, name.name, timestamp_str)
			},
			_ => (),
		},
		_ => (),
	}
}
