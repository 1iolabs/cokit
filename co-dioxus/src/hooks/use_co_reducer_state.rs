// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
