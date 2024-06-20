use super::core_state_or_default;
use crate::{CoStorage, CO_CORE_NAME_CO};
use co_core_co::{Co, Participant};
use co_identity::{IdentityBox, IdentityResolver};
use co_primitives::OptionLink;
use co_storage::StorageError;
use futures::{stream, StreamExt, TryStreamExt};

/// Read participants from a CO.
pub async fn participants(storage: &CoStorage, co_state: OptionLink<Co>) -> Result<Vec<Participant>, StorageError> {
	let co: Co = core_state_or_default(storage, co_state, CO_CORE_NAME_CO).await?;
	Ok(co.participants.into_values().collect())
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
