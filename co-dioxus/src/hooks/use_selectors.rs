// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{hooks::use_target_resource::use_target_resource_cleared, CoBlockStorage, CoError, Cos};
use co_sdk::{CoId, CoReducerState};
use dioxus::{
	hooks::Resource,
	signals::{ReadableExt, SyncSignal},
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

type SelectorTargets = Vec<SyncSignal<Option<Result<CoReducerState, CoError>>>>;

/// The COs a selection runs for, in the order they were requested.
///
/// Signal equality is storage identity, so this differs as soon as a CO is added, removed or
/// replaced, the requested order changes, or one of the COs is served by another attachment.
fn selector_targets(cos: &Cos) -> SelectorTargets {
	cos.iter().map(|co| co.reducer_state).collect()
}

/// Select state using multiple COs' block storages.
///
/// Changing the COs selects again and drops the previous selection, so a signal taken from
/// `Resource::suspend` must not outlive the change in a child component or a memo.
pub fn use_selectors<F, Fut, T>(cos: &Cos, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(Vec<CoSelector>) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_target_resource_cleared(selector_targets(cos), {
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
///
/// Changing the COs selects again and drops the previous selection, so a signal taken from
/// `Resource::suspend` must not outlive the change in a child component or a memo.
pub fn use_selector_states<F, Fut, T>(cos: &Cos, f: F) -> Resource<Result<T, CoError>>
where
	F: Fn(Vec<CoSelectorState>) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	use_target_resource_cleared(selector_targets(cos), {
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
