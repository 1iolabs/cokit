// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
	#[arg(long, default_value_t = CO_CORE_NAME_KEYSTORE.to_string())]
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
