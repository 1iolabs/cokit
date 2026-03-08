// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{library::co_actor::CoActor, use_co_context, Co, CoBlockStorage};
use co_actor::Actor;
use co_sdk::CoId;
use dioxus::prelude::*;
use std::ops::Deref;

/// Use multiple COs at once.
pub fn use_cos(cos: ReadSignal<Vec<CoId>>) -> Cos {
	let context = use_co_context();
	use_hook(move || {
		let cos = cos();
		let items = cos
			.into_iter()
			.map(|co_id| {
				let reducer_state = SyncSignal::new_maybe_sync(None);
				let last_error = SyncSignal::new_maybe_sync(Ok(()));
				let actor_spawner = Actor::spawner(Default::default(), CoActor::new(co_id.clone())).expect("actor");
				let handle = actor_spawner.handle();
				context.execute_future_parallel(move |application| async move {
					actor_spawner.spawn(application.context().tasks(), (application.context().clone(), reducer_state));
				});
				let storage = CoBlockStorage::new(handle.clone(), None);
				Co { co_id, last_error, context: context.clone(), reducer_state, handle, storage }
			})
			.collect();
		Cos(items)
	})
}

#[derive(Debug, Clone)]
pub struct Cos(Vec<Co>);

impl Deref for Cos {
	type Target = Vec<Co>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
