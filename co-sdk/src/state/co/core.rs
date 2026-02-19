// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	state::{query_core, Query, QueryError},
	CO_CORE_NAME_CO,
};
use cid::Cid;
use co_core_co::Co;
use co_primitives::{AnyBlockStorage, CoreName, OptionLink};
use serde::de::DeserializeOwned;

/// Get core by name. Fails if not exists.
pub async fn core<T>(storage: &impl AnyBlockStorage, co_state: OptionLink<Co>, name: &str) -> Result<T, QueryError>
where
	T: Default + DeserializeOwned + Clone + Send + Sync + 'static,
{
	query_core(CoreName::<T>::new(name)).execute(storage, co_state).await
}

/// Get core by name. Returns default if not exists.
pub async fn core_or_default<T>(
	storage: &impl AnyBlockStorage,
	co_state: OptionLink<Co>,
	name: &str,
) -> Result<T, QueryError>
where
	T: Default + DeserializeOwned + Clone + Send + Sync + 'static,
{
	query_core(CoreName::<T>::new(name))
		.with_default()
		.execute(storage, co_state)
		.await
}

/// Get the state of a core by name.
/// Returns [`None`] is core can not be found.
pub async fn core_state(
	storage: &impl AnyBlockStorage,
	co_state: OptionLink<Co>,
	name: &str,
) -> Result<Option<Cid>, QueryError> {
	if CO_CORE_NAME_CO == name {
		Ok(co_state.into())
	} else {
		let co = query_core(CO_CORE_NAME_CO).with_default().execute(storage, co_state).await?;
		Ok(co.cores.get(name).and_then(|core| core.state))
	}
}
