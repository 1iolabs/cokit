use super::{query_core, Query, QueryError};
use crate::{state, CoStorage, CO_CORE_NAME_CO};
use co_core_co::Co;
use co_primitives::{Network, OptionLink};
use futures::TryStreamExt;

/// Read network settings from an CO.
pub async fn networks(storage: &CoStorage, co_state: OptionLink<Co>) -> Result<Vec<Network>, QueryError> {
	let co = query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?;
	Ok(state::stream(storage.clone(), &co.network).try_collect().await?)
}
