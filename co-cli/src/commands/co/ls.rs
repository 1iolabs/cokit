// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, library::cli_context::CliContext};
use co_sdk::{state::memberships, CoId};
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
	let stream = memberships(local_co_reducer.storage(), local_co_reducer.reducer_state().await.co());
	pin_mut!(stream);
	while let Some(item) = stream.next().await {
		match item {
			Ok((id, did, tags, membership_state)) => {
				println!("{id} | {did} | {tags} | {membership_state:?}")
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
