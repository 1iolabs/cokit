// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::services::discovery::{
	action::{ConnectAction, DiscoveryAction, TimeoutAction},
	actor::DiscoveryContext,
	state::DiscoveryState,
};
use co_actor::{time, Actions, Epic};
use futures::{FutureExt, Stream};

/// Spawns a timeout future for each `Connect` action.
/// When the timeout expires, dispatches a `Timeout` action.
/// TODO: Should this be conditional? Only dispatch when no connection could be made?
pub struct TimeoutEpic;
impl TimeoutEpic {
	pub fn new() -> Self {
		Self
	}
}
impl Epic<DiscoveryAction, DiscoveryState, DiscoveryContext> for TimeoutEpic {
	fn epic(
		&mut self,
		_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
		action: &DiscoveryAction,
		state: &DiscoveryState,
		_context: &DiscoveryContext,
	) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
		let DiscoveryAction::Connect(ConnectAction { id, .. }) = action else {
			return None;
		};

		let id = *id;
		let timeout = state.timeout;

		Some(
			async move {
				time::sleep(timeout).await;
				Ok(DiscoveryAction::Timeout(TimeoutAction { id }))
			}
			.into_stream(),
		)
	}
}
