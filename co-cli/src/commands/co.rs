use crate::cli::{Cli, APP_IDENTIFIER};
use co_sdk::{memberships, ApplicationBuilder};
use exitcode::ExitCode;
use futures::{pin_mut, stream::StreamExt};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO Command
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Commands {
	/// List all local COs.
	Ls,

	/// Print block binary data.
	Cat(CatCommand),
}

#[derive(Debug, Clone, clap::Args)]
pub struct CatCommand {
	/// CO ID
	pub co: String,

	/// The CID to print.
	pub cid: String,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	match &command.command {
		Commands::Ls => ls(cli).await,
		Commands::Cat(cat_command) => cat(cat_command).await,
	}
}

async fn ls(cli: &Cli) -> Result<ExitCode, anyhow::Error> {
	// application
	let mut application_builder = match &cli.base_path {
		None => ApplicationBuilder::new(APP_IDENTIFIER.to_owned()),
		Some(path) => ApplicationBuilder::new_with_path(APP_IDENTIFIER.to_owned(), path.clone()),
	};
	if cli.no_log == false {
		application_builder = application_builder.with_bunyan_logging(cli.log_path.clone());
	}
	let application = application_builder.build().await.expect("application");

	// local
	let local_co_reducer: co_sdk::CoReducer = application.create_local_co(!cli.no_keychain).await.expect("local-co");

	// list
	let mut result = exitcode::OK;
	let stream = memberships(local_co_reducer.clone());
	pin_mut!(stream);
	while let Some(item) = stream.next().await {
		match item {
			Ok((id, state, tags)) => {
				println!("{} | {} | {}", id, &state.map(|i| i.to_string()).unwrap_or_default(), tags)
			},
			Err(e) => {
				result = exitcode::UNAVAILABLE;
				eprintln!("{:?}", e);
			},
		}
	}

	// result
	Ok(result)
}

async fn cat(command: &CatCommand) -> Result<ExitCode, anyhow::Error> {
	Ok(exitcode::OK)
}
