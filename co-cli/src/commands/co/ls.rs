// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
