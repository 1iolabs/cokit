// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::connections::{
	action::{ConnectionAction, NetworkResolveAction, NetworkResolveCompleteAction},
	actor::ConnectionsContext,
	ConnectionState, NetworkResolver,
};
use co_actor::{time::Instant, Actions, Epic};
use futures::{FutureExt, Stream};
pub struct NetworkResolveEpic();
impl NetworkResolveEpic {
	pub fn new() -> Self {
		Self()
	}
}
impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> for NetworkResolveEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
		message: &ConnectionAction,
		_state: &ConnectionState,
		context: &ConnectionsContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::NetworkResolve(NetworkResolveAction { id }) => {
				let context = context.clone();
				let id = id.clone();
				Some(
					async move {
						let result = context
							.network_resolver
							.networks(id.clone())
							.await
							.map_err(|err| format!("Resolve failed: {err}"));
						Ok(ConnectionAction::NetworkResolveComplete(NetworkResolveCompleteAction {
							id,
							result,
							time: Instant::now(),
						}))
					}
					.into_stream(),
				)
			},
			_ => None,
		}
	}
}
