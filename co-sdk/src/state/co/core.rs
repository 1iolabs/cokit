use crate::state::{query_core, Query, QueryError};
use co_core_co::Co;
use co_primitives::{AnyBlockStorage, CoreName, OptionLink};
use serde::de::DeserializeOwned;

/// Get core by name. Fails if not exists.
pub async fn core<T>(storage: &impl AnyBlockStorage, co_state: OptionLink<Co>, name: &str) -> Result<T, QueryError>
where
	T: Default + DeserializeOwned + Clone + Send + Sync + 'static,
{
	Ok(query_core(CoreName::<T>::new(name)).execute(storage, co_state).await?)
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
	Ok(query_core(CoreName::<T>::new(name))
		.with_default()
		.execute(storage, co_state)
		.await?)
}
