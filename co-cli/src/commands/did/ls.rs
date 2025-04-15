use super::Command as DidCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use co_sdk::{
	state::{self, Identity},
	CoId, CoReducerFactory, CO_CORE_NAME_KEYSTORE, CO_ID_LOCAL,
};
use exitcode::ExitCode;
use futures::TryStreamExt;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// The CO ID.
	#[arg(long, default_value_t = CoId::from(CO_ID_LOCAL))]
	pub co: CoId,

	/// The COre Name.
	#[arg(long, default_value_t = String::from(CO_CORE_NAME_KEYSTORE))]
	pub core: String,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	_did_command: &DidCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let co_reducer = application.context().try_co_reducer(&command.co).await?;
	let identities: Vec<Identity> =
		state::identities(co_reducer.storage(), co_reducer.reducer_state().await.co(), Some(&command.core))
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
