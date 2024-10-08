use super::{
	epics::connect::ConnectEpic, CoConnection, ConnectionAction, ConnectionState, NetworkConnection, NetworkWithContext,
};
use crate::{
	actor::{Actor, ActorError, ActorHandle, Epic, EpicRuntime},
	CoContext,
};
use async_trait::async_trait;
use co_primitives::Tags;
use std::time::Duration;

pub struct ConnectionActorState<E> {
	connection: ConnectionState,
	epic: EpicRuntime<E, ConnectionAction, ConnectionState, CoContext>,
}

// Actor::spawn(EpicActor::new(Connections {}), (ConnectEpic {}, context, ()))

pub struct Connections {
	context: CoContext,
	keep_alive: Duration,
}
#[async_trait]
impl Actor for Connections {
	type Message = ConnectionAction;
	type State = ConnectionActorState<ConnectEpic>;
	type Initialize = ();

	async fn initialize(&self, _tags: Tags, _initialize: Self::Initialize) -> Result<Self::State, ActorError> {
		Ok(ConnectionActorState {
			connection: ConnectionState {
				cache: Default::default(),
				co: Default::default(),
				networks: Default::default(),
			},
			epic: EpicRuntime::new(ConnectEpic {}, |err| {
				tracing::error!(?err, "connections-epic-error");
				None
			}),
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		// state
		match &message {
			// TODO: use did? create new connections when different to existing?
			ConnectionAction::Use(co, did, time, networks) => {
				let mut networks = networks.clone();

				// co connections
				match state.connection.co.get_mut(co) {
					Some(co_connection) => {
						// clear networks already in use
						networks.retain(|network| !co_connection.networks.contains(network));

						// insert networks
						for network in networks.iter() {
							co_connection.networks.insert(network.clone());
						}
					},
					None => {
						// insert co
						state.connection.co.insert(
							co.clone(),
							CoConnection {
								id: co.clone(),
								from: did.clone(),
								keep_alive: *time + self.keep_alive,
								networks: networks.clone(),
							},
						);
					},
				}

				// network connections
				for network in networks.iter() {
					// networks: get/create
					match state.connection.networks.get_mut(network) {
						Some(network_connection) => {
							// reference
							network_connection.references = network_connection.references + 1;
							network_connection.keep_alive = *time + self.keep_alive;
						},
						None => {
							// insert
							state.connection.networks.insert(
								network.clone(),
								NetworkConnection {
									keep_alive: *time + self.keep_alive,
									network: network.clone(),
									peers: Default::default(),
									references: 1,
								},
							);

							// connect
							handle.dispatch(ConnectionAction::Connect(NetworkWithContext {
								network: network.clone(),
								from: did.clone(),
							}))?;
						},
					};
				}
			},
			ConnectionAction::Connected(network, connected) => {
				if let Some(network) = state.connection.networks.get_mut(network) {
					match connected {
						Ok(peers) => {
							network.peers = peers.clone();
							// TODO: update co use handles
						},
						Err(err) => {
							tracing::warn!(?err, "connections-failed");
							// TODO: handle
						},
					}
				}
			},
			ConnectionAction::Disconnected(network, _reason) => {
				if let Some(network_connection) = state.connection.networks.get_mut(network) {
					// remove references
					for (_, co_connection) in state.connection.co.iter_mut() {
						if co_connection.networks.remove(network) {
							// references
							network_connection.references = network_connection.references - 1;

							// handle
							if co_connection.networks.is_empty() {
								// TODO: update co use handles
							}
						}
					}

					// remove disconnected
					state
						.connection
						.co
						.retain(|_, co_connection| !co_connection.networks.is_empty());
				}
			},
			_ => {},
		}

		// epics
		state.epic.handle(handle, &message, &state.connection, &self.context);

		// result
		Ok(())
	}
}

#[cfg(test)]
mod tests {

	#[tokio::test]
	async fn test_use() {}
}
