use crate::{Co, CoError};
use co_sdk::CoReducerState;
use dioxus::{
	hooks::{use_resource, Resource},
	signals::ReadableExt,
};
use futures::future::Either;

/// Subscribe to a reducer state as a resource.
pub fn use_co_reducer_state(co: &Co) -> Resource<Result<CoReducerState, CoError>> {
	use_resource({
		let co = co.clone();
		move || {
			let reducer_state = match co.reducer_state.cloned() {
				Some(reducer_state) => Either::Left(reducer_state),
				None => Either::Right(co.clone()),
			};
			async move {
				match reducer_state {
					Either::Left(reducer_state) => reducer_state,
					Either::Right(co) => co.reducer_state().await,
				}
			}
		}
	})
}
