use crate::{cli::Cli, library::cli_context::CliContext};
use co_sdk::{memberships, CoId};
use exitcode::ExitCode;
use futures::{pin_mut, stream::StreamExt};

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// The CID to print.
	/// If not specified using the root state.
	pub cid: Option<String>,

	/// Pretty print data.
	#[arg(short, long)]
	pub pretty: bool,
}

pub async fn command(context: &CliContext, cli: &Cli) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let local_co_reducer = application.local_co_reducer().await?;

	// list
	let mut result = exitcode::OK;
	let stream = memberships(local_co_reducer.clone());
	pin_mut!(stream);
	while let Some(item) = stream.next().await {
		match item {
			Ok((id, state, tags)) => {
				println!("{} | {} | {}", id, state, tags)
			},
			Err(e) => {
				result = exitcode::UNAVAILABLE;
				eprintln!("error: {:?}", e);
			},
		}
	}

	// result
	Ok(result)
}
