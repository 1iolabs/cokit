use crate::{CoReducerError, CoStorage, CO_CORE_NAME_CO};
use cid::Cid;
use co_primitives::OptionLink;
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use serde::de::DeserializeOwned;

/// Return core state (CID and actual state) from an CO assuming the `co_state` points to the root of a `co-core-co`
/// core.
///
/// ## Errors
/// - `CoReducerError::CoreNotFound` - If the core not exists.
pub async fn core_state<T: DeserializeOwned + Send + Sync + Default + Clone + 'static, S: BlockStorage + 'static>(
	storage: &S,
	co_state: OptionLink<co_core_co::Co>,
	core_name: &str,
) -> Result<(OptionLink<T>, T), CoReducerError> {
	// co?
	if core_name == CO_CORE_NAME_CO {
		if let Some(state_cid) = co_state.cid() {
			return Ok((state_cid.into(), storage.get_deserialized(state_cid).await?));
		}
		return Ok((None.into(), T::default()));
	}

	// other
	let co_state: co_core_co::Co = if let Some(state_cid) = co_state.cid() {
		storage.get_deserialized(state_cid).await?
	} else {
		co_core_co::Co::default()
	};
	if let Some(core) = co_state.cores.get(core_name) {
		if let Some(core_state) = &core.state {
			return Ok((core_state.into(), storage.get_deserialized(core_state).await?));
		} else {
			return Ok((None.into(), T::default()));
		}
	}

	// not found
	Err(CoReducerError::CoreNotFound(core_name.to_owned()))
}

/// Return the core state or default is the core not exists.
pub async fn core_state_or_default<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
	storage: &CoStorage,
	co_state: OptionLink<co_core_co::Co>,
	core_name: &str,
) -> Result<T, StorageError> {
	match core_state(storage, co_state, core_name).await {
		Ok((_, core_state)) => Ok(core_state),
		Err(CoReducerError::CoreNotFound(_)) => Ok(T::default()),
		Err(CoReducerError::Storage(err)) => Err(err)?,
	}
}

/// Return core state reference from an CO.
pub async fn core_state_reference(
	storage: &CoStorage,
	co_state: OptionLink<co_core_co::Co>,
	core_name: &str,
) -> Result<Option<Cid>, CoReducerError> {
	// co?
	if core_name == CO_CORE_NAME_CO {
		return Ok(*co_state.cid());
	}

	// other
	let co_state: co_core_co::Co = if let Some(state_cid) = co_state.cid() {
		storage.get_deserialized(state_cid).await?
	} else {
		co_core_co::Co::default()
	};
	if let Some(core) = co_state.cores.get(core_name) {
		if let Some(core_state) = &core.state {
			return Ok(Some(*core_state));
		}
	}

	// not found
	Err(CoReducerError::CoreNotFound(core_name.to_owned()))
}
