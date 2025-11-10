use crate::services::connections::{
	action::{ConnectionAction, NetworkResolveAction, NetworkResolvedAction},
	actor::ConnectionsContext,
	ConnectionState, NetworkResolver,
};
use co_actor::{Actions, Epic};
use futures::{FutureExt, Stream};
use std::time::Instant;

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
						Ok(ConnectionAction::NetworkResolved(NetworkResolvedAction {
							id: id.clone(),
							result: context
								.network_resolver
								.networks(id)
								.await
								.map_err(|err| format!("Resolve failed: {err}")),
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
