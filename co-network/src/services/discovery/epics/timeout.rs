// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::discovery::{
	action::{ConnectAction, DiscoveryAction, TimeoutAction},
	actor::DiscoveryContext,
	state::DiscoveryState,
};
use co_actor::{time, Actions, Epic};
use futures::{FutureExt, Stream};

/// Spawns a timeout future for each `Connect` action.
/// When the timeout expires, dispatches a `Timeout` action.
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
