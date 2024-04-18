use crate::{CoReducerError, CoStorage, CO_CORE_NAME_CO};
use co_primitives::OptionLink;
use co_storage::BlockStorageExt;
use libipld::Cid;
use serde::de::DeserializeOwned;

/// Return core state (CID and actual state) from an CO assuming the `co_state` points to the root of a `co-core-co`
/// core.
///
/// ## Errors
/// - `CoReducerError::CoreNotFound` - If the core not exists.
pub async fn core_state<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
	storage: &CoStorage,
	co_state: OptionLink<co_core_co::Co>,
	core: &str,
) -> Result<(Option<Cid>, T), CoReducerError> {
	// co?
	if core == CO_CORE_NAME_CO {
		if let Some(state_cid) = co_state.cid() {
			return Ok((Some(*state_cid), storage.get_deserialized(state_cid).await?))
		}
		return Ok((None, T::default()));
	}

	// other
	let co_state: co_core_co::Co = if let Some(state_cid) = co_state.cid() {
		storage.get_deserialized(state_cid).await?
	} else {
		co_core_co::Co::default()
	};
	if let Some(core) = co_state.cores.get(core) {
		if let Some(core_state) = &core.state {
			return Ok((Some(*core_state), storage.get_deserialized(core_state).await?));
		} else {
			return Ok((None, T::default()));
		}
	}

	// not found
	return Err(CoReducerError::CoreNotFound(core.to_owned()));
}

/// Return the core state or default is the core not exists.
pub async fn core_state_or_default<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
	storage: &CoStorage,
	co_state: OptionLink<co_core_co::Co>,
	core: &str,
) -> Result<T, CoReducerError> {
	match core_state(&storage, co_state, core).await {
		Ok((_, core_state)) => Ok(core_state),
		Err(CoReducerError::CoreNotFound(_)) => Ok(T::default()),
		Err(e) => Err(e)?,
	}
}
