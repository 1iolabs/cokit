// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{hooks::use_target_resource::use_target_resource_cleared, Co, CoError};
use co_sdk::CoReducerState;
use dioxus::{hooks::Resource, signals::ReadableExt};
use futures::future::Either;

/// Subscribe to a reducer state as a resource.
///
/// Switching COs reads again and drops the previous state, so a signal taken from
/// `Resource::suspend` must not outlive the switch in a child component or a memo.
pub fn use_co_reducer_state(co: &Co) -> Resource<Result<CoReducerState, CoError>> {
	use_target_resource_cleared(co.reducer_state, {
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
