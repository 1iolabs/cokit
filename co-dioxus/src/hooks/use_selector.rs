use crate::{Co, CoBlockStorage, CoError};
use co_sdk::CoReducerState;
use dioxus::{
	hooks::{use_resource, Resource},
	signals::ReadableExt,
};
use std::future::Future;

/// Select state using the Co's block storage.
pub fn use_selector<F, Fut, T>(co: &Co, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(CoBlockStorage) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_resource({
		let co = co.clone();
		move || {
			let f = f(co.storage());
			async move { f.await.map_err(CoError::from) }
		}
	})
}

/// Select state using the Co's block storage.
pub fn use_selector_state<F, Fut, T>(co: &Co, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(CoBlockStorage, CoReducerState) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_resource({
		let co = co.clone();
		move || {
			let co = co.clone();
			let f = f.clone();
			async move {
				f(
					co.storage(),
					match co.reducer_state.cloned() {
						Some(reducer_state) => reducer_state?,
						None => co.reducer_state().await?,
					},
				)
				.await
				.map_err(CoError::from)
			}
		}
	})
}
