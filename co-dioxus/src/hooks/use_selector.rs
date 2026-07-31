// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{hooks::use_target_resource::use_target_resource_cleared, Co, CoBlockStorage, CoError};
use co_sdk::CoReducerState;
use dioxus::{hooks::Resource, signals::ReadableExt};
use std::future::Future;

/// Select state using the Co's block storage.
///
/// Switching COs selects again and drops the previous selection, so a signal taken from
/// `Resource::suspend` must not outlive the switch in a child component or a memo.
pub fn use_selector<F, Fut, T>(co: &Co, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(CoBlockStorage) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_target_resource_cleared(co.reducer_state, {
		let co = co.clone();
		move || {
			let f = f(co.storage());
			async move { f.await.map_err(CoError::from) }
		}
	})
}

/// Select state using the Co's block storage.
///
/// Switching COs selects again and drops the previous selection, so a signal taken from
/// `Resource::suspend` must not outlive the switch in a child component or a memo.
pub fn use_selector_state<F, Fut, T>(co: &Co, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(CoBlockStorage, CoReducerState) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_target_resource_cleared(co.reducer_state, {
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
