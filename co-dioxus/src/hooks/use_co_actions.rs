// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{hooks::use_co_error::use_co_error, CoContext};
use co_primitives::ReducerAction;
use co_sdk::{Application, BlockStorageExt, CoId, CoReducerFactory, CoReducerState};
use dioxus::prelude::*;
use futures::{StreamExt, TryStreamExt};
use serde::de::{DeserializeOwned, IgnoredAny};

pub fn use_co_actions<T>(
	co: ReadSignal<CoId>,
	state: ReadSignal<CoReducerState, SyncStorage>,
	core: Option<String>,
	skip: usize,
	limit: usize,
) -> Resource<Vec<ReducerAction<T>>>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	let context: CoContext = use_context();
	let error = use_co_error();
	use_resource(use_reactive!(|(core, skip, limit)| {
		let context = context.clone();
		async move {
			let state = state.cloned();
			let co = co();
			context
				.result(error, move |application| async move {
					read_actions(&application, &co, state, &core, skip, limit).await
				})
				.await
				.unwrap_or_default()
		}
	}))
}

async fn read_actions<T>(
	application: &Application,
	co: &CoId,
	state: CoReducerState,
	core: &Option<String>,
	skip: usize,
	limit: usize,
) -> Result<Vec<ReducerAction<T>>, anyhow::Error>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	let reducer = application.co().try_co_reducer(co).await?;
	let storage = reducer.storage();
	if state.is_empty() {
		return Ok(Vec::new());
	}
	let items: Vec<_> = application
		.co()
		.entries_from_heads(co, storage.clone(), state.1)
		.await?
		.try_filter_map({
			let storage = storage.clone();
			move |entry| {
				let storage = storage.clone();
				async move {
					Ok(if let Some(query_core) = core {
						let action: ReducerAction<IgnoredAny> =
							storage.get_deserialized(&entry.entry().payload).await?;
						if &action.core == query_core {
							Some(entry)
						} else {
							None
						}
					} else {
						Some(entry)
					})
				}
			}
		})
		.skip(skip)
		.take(limit)
		.map_ok(|entry| entry.entry().payload)
		.and_then(move |action_cid| {
			let storage = storage.clone();
			async move {
				let action: ReducerAction<T> = storage.get_deserialized(&action_cid).await?;
				Ok(action)
			}
		})
		.try_collect()
		.await?;
	Ok(items)
}
