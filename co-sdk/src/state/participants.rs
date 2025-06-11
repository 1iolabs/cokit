use super::{query_core, Query, QueryError};
use crate::{CoStorage, CO_CORE_NAME_CO};
use co_core_co::{Co, Participant};
use co_identity::{IdentityBox, IdentityResolver};
use co_primitives::{Did, OptionLink};
use futures::{stream, StreamExt, TryStreamExt};

/// Read participants from a CO.
pub async fn participants(storage: &CoStorage, co_state: OptionLink<Co>) -> Result<Vec<Participant>, QueryError> {
	let co = query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?;
	Ok(co.participants.into_values().collect())
}

/// Read active participants from a CO.
pub async fn participants_active(
	storage: &CoStorage,
	co_state: OptionLink<Co>,
) -> Result<Vec<Participant>, QueryError> {
	Ok(participants(storage, co_state)
		.await?
		.into_iter()
		.filter(|participant| participant.state.is_active())
		.collect())
}

/// Read participant identities from a CO.
pub async fn participant_identities<R: IdentityResolver + Send + Sync + 'static>(
	identity_resolver: &R,
	storage: &CoStorage,
	co_state: OptionLink<Co>,
) -> Result<Vec<IdentityBox>, anyhow::Error> {
	Ok(stream::iter(participants(storage, co_state).await?)
		.then(|participant| async move { identity_resolver.resolve(&participant.did).await })
		.try_collect()
		.await?)
}

/// Test if `participant` is a CO participant.
/// If the CO is public this is always true.
/// TODO: Permissions: This should be handled by co permissions core.
pub async fn is_participant(
	storage: &CoStorage,
	co_state: OptionLink<Co>,
	participant: &Option<Did>,
) -> anyhow::Result<bool> {
	let co = query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?;
	if co.keys.is_none() {
		return Ok(true);
	}
	if let Some(participant) = participant {
		Ok(co
			.participants
			.iter()
			.any(|item| item.1.state.has_access() && item.0 == participant))
	} else {
		Ok(false)
	}
}
