use super::core_state_or_default;
use crate::{state, CoStorage, CO_CORE_NAME_CO};
use co_core_co::Co;
use co_primitives::{Network, OptionLink};
use co_storage::StorageError;
use futures::TryStreamExt;

/// Read network settings from an CO.
pub async fn networks(storage: &CoStorage, co_state: OptionLink<Co>) -> Result<Vec<Network>, StorageError> {
	let co: Co = core_state_or_default(storage, co_state, CO_CORE_NAME_CO).await?;
	Ok(state::stream(storage.clone(), &co.network).try_collect().await?)
}
