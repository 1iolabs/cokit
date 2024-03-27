use crate::cli::Cli;
use co_sdk::CoId;
use exitcode::ExitCode;

mod create;
mod edit;
mod get;
mod send;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// ID of the co
	pub co_id: CoId,

	/// ID of the room
	room_id: String,

	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	Create(create::Command),
	Send(send::Command),
	Get(get::Command),
	Edit(create::Command),
}

pub async fn command(cli: &Cli, room_command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &room_command.command {
		Commands::Create(command) => create::command(cli, room_command, command).await,
		Commands::Send(command) => send::command(cli, room_command, command).await,
		Commands::Get(command) => get::command(cli, room_command, command).await,
		Commands::Edit(command) => edit::command(cli, room_command, command).await,
	}
}
