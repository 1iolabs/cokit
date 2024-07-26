use super::Command as DidCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::anyhow;
use co_sdk::state::{self, Identity};
use exitcode::ExitCode;
use futures::TryStreamExt;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	did_command: &DidCommand,
	_command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let co_reducer = application
		.co_reducer(&did_command.co)
		.await?
		.ok_or(anyhow!("Co not found: {}", did_command.co))?;
	let identities: Vec<Identity> =
		state::identities(co_reducer.storage(), co_reducer.co_state().await, Some(&did_command.core))
			.try_collect()
			.await?;

	// print
	println!("total {}", identities.len());
	println!("NAME | DID | DESCRIPTION");
	println!("-----|-----|------------");
	for identity in identities {
		println!("{} | {} | {}", identity.name, identity.did, identity.description);
	}

	// result
	Ok(exitcode::OK)
}
