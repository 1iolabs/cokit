use crate::{cli::Cli, library::application::application};
use co_core_keystore::KeyStoreAction;
use co_core_membership::MembershipsAction;
use co_sdk::{find_memberships, CoId, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP};
use exitcode::ExitCode;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID
	pub co: CoId,

	/// DID
	pub did: Option<String>,
}

pub async fn command(cli: &Cli, command: &Command) -> Result<ExitCode, anyhow::Error> {
	let application = application(cli).await?;
	let local = application.local_co_reducer().await?;
	let identity = application.local_identity();

	// membership
	let mut memberships = find_memberships(&local, &command.co).await?;
	if let Some(did) = &command.did {
		memberships = memberships.into_iter().filter(|item| &item.did == did).collect();
	}

	// remove
	for membership in memberships {
		// log
		tracing::info!(co = ?membership.id, did = membership.did, "remove-co");

		// remove membership
		local
			.push(
				&identity,
				CO_CORE_NAME_MEMBERSHIP,
				&MembershipsAction::Remove { id: membership.id, did: Some(membership.did) },
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
