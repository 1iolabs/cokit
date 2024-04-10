use crate::{use_co_selector, CoStateResult};
use co_sdk::state::core_state;
use dioxus::prelude::*;
use serde::de::DeserializeOwned;

pub fn use_co_state<T>(co: &str, core: &str) -> Signal<CoStateResult<T>, SyncStorage>
where
	T: DeserializeOwned + Send + Sync + Default + Clone + 'static,
{
	let core = core.to_owned();
	use_co_selector(co, move |storage, co_state| {
		let core = core.clone();
		async move { Ok(core_state::<T>(&storage, co_state.into(), &core).await?.1) }
	})
}
