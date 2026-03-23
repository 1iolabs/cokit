// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{cli::Cli, library::cli_context::CliContext};
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_sdk::{find_membership_by, CoId, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// DID
	pub did: Option<String>,
}

pub async fn command(context: &CliContext, cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = context.application(cli).await;
	let local = application.local_co_reducer().await?;
	let identity = application.local_identity();

	// membership
	let membership = find_membership_by(&local, &command.co, command.did.as_ref(), None).await?;
	if let Some(membership) = membership {
		// log
		tracing::info!(co = ?membership.id, did = ?command.did, ?membership, "remove-co");

		// remove membership
		local
			.push(
				&identity,
				CO_CORE_NAME_MEMBERSHIP,
				&MembershipsAction::Remove { id: membership.id, did: command.did.clone() },
			)
			.await?;

		// remove key
		if let Some(key) = &membership.key {
			local
				.push(&identity, CO_CORE_NAME_KEYSTORE, &KeyStoreAction::Remove(key.to_owned()))
				.await?;
		}
	}

	// result
	Ok(exitcode::OK)
}
