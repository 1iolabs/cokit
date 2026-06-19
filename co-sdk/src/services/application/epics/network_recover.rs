// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{Action, ActionError, CoContext};
use co_actor::Actions;
use futures::{FutureExt, Stream};

/// Recover the network on demand by injecting a recovery task into the running swarm.
/// Mirrors `network_start`.
pub fn network_recover(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::NetworkRecover => {
			let context = context.clone();
			Some(
				async move {
					let result = match context.network().await {
						Some(network) => network.recover().await.map_err(ActionError::from),
						None => Err(ActionError::from(anyhow::anyhow!("network not started"))),
					};
					Ok(Action::NetworkRecoverComplete(result))
				}
				.into_stream(),
			)
		},
		_ => None,
	}
}
