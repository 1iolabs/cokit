// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{CoBlockStorage, CoError, Cos};
use co_sdk::{CoId, CoReducerState};
use dioxus::{
	hooks::{use_resource, Resource},
	signals::ReadableExt,
};
use std::future::Future;

pub struct CoSelector {
	pub co: CoId,
	pub storage: CoBlockStorage,
}

pub struct CoSelectorState {
	pub co: CoId,
	pub storage: CoBlockStorage,
	pub state: CoReducerState,
}

/// Select state using multiple COs' block storages.
pub fn use_selectors<F, Fut, T>(cos: &Cos, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(Vec<CoSelector>) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_resource({
		let cos = cos.clone();
		move || {
			let selectors: Vec<CoSelector> =
				cos.iter().map(|co| CoSelector { co: co.co(), storage: co.storage() }).collect();
			let f = f(selectors);
			async move { f.await.map_err(CoError::from) }
		}
	})
}

/// Select state using multiple COs' block storages and reducer states.
pub fn use_selector_states<F, Fut, T>(cos: &Cos, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(Vec<CoSelectorState>) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_resource({
		let cos = cos.clone();
		move || {
			let cos = cos.clone();
			let f = f.clone();
			async move {
				let mut selector_states = Vec::with_capacity(cos.len());
				for co in cos.iter() {
					let state = match co.reducer_state.cloned() {
						Some(reducer_state) => reducer_state?,
						None => co.reducer_state().await?,
					};
					selector_states.push(CoSelectorState { co: co.co(), storage: co.storage(), state });
				}
				f(selector_states).await.map_err(CoError::from)
			}
		}
	})
}
