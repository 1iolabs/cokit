// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{
	action::{ConnectionAction, DidPeersChangedAction, PeersChangedAction},
	epics::epic,
	CoConnectionOverview, ConnectionMessage, ConnectionOverview, ConnectionState,
};
use crate::{
	services::{
		connections::{library::bootstrap_from_multiaddrs::bootstrap_from_multiaddrs, resolve::DynamicNetworkResolver},
		discovery::DiscoveryApi,
		network::CoNetworkTaskSpawner,
	},
	NetworkSettings,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, Reducer, ResponseStream, ResponseStreams, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use co_primitives::{CoId, Did, DynamicCoDate, Tags};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub struct ConnectionsContext {
	pub tasks: TaskSpawner,
	pub settings: NetworkSettings,
	pub network: CoNetworkTaskSpawner,
	pub identity_resolver: IdentityResolverBox,
	pub private_identity_resolver: PrivateIdentityResolverBox,
	pub network_resolver: DynamicNetworkResolver,
	pub date: DynamicCoDate,
	pub discovery: DiscoveryApi,
}

pub struct State {
	state: ConnectionState,
	epic: EpicRuntime<ConnectionMessage, ConnectionAction, ConnectionState, ConnectionsContext>,
	peers_changed: BTreeMap<CoId, ResponseStreams<PeersChangedAction>>,
	did_peers_changed: BTreeMap<Did, ResponseStreams<DidPeersChangedAction>>,
	overviews: ResponseStreams<ConnectionOverview>,
	co_overviews: HashMap<CoId, ResponseStreams<CoConnectionOverview>>,
}

pub struct Connections {
	context: ConnectionsContext,
}
impl Connections {
	pub fn new(context: ConnectionsContext) -> Self {
		Self { context }
	}
}
#[async_trait]
impl Actor for Connections {
	type Message = ConnectionMessage;
	type State = State;
	type Initialize = ();

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(State {
			state: ConnectionState {
				keep_alive: self.context.settings.keep_alive,
				co: Default::default(),
				did: Default::default(),
				networks: Default::default(),
				peers: Default::default(),
				bootstrap: bootstrap_from_multiaddrs(self.context.settings.bootstrap.clone())?,
			},
			epic: EpicRuntime::new(epic(tags.clone()), |err| {
				tracing::error!(?err, "connection-epic-error");
				None
			}),
			peers_changed: Default::default(),
			did_peers_changed: Default::default(),
			overviews: Default::default(),
			co_overviews: Default::default(),
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		// state
		let (action, response) = match message {
			ConnectionMessage::Use(action, response) => {
				let co = action.id.clone();
				(ConnectionAction::Use(action), Some(ResponseKind::Co(co, response)))
			},
			ConnectionMessage::DidUse(action, response) => {
				let did = action.to.clone();
				(ConnectionAction::DidUse(action), Some(ResponseKind::Did(did, response)))
			},
			ConnectionMessage::Action(action) => (action, None),
			ConnectionMessage::Overview(response) => {
				response.respond(ConnectionOverview::from(&state.state));
				return Ok(());
			},
			ConnectionMessage::OverviewStream(mut stream) => {
				// send the current state immediately, then keep the subscriber for
				// the post-reduce broadcast below (dropped automatically when closed).
				stream.send(ConnectionOverview::from(&state.state)).ok();
				state.overviews.push(stream);
				return Ok(());
			},
			ConnectionMessage::CoOverview(co, response) => {
				response.respond(CoConnectionOverview::from_state(&state.state, &co));
				return Ok(());
			},
			ConnectionMessage::CoOverviewStream(co, mut stream) => {
				// send the current CO-scoped overview immediately, then keep the
				// subscriber under its CO for the post-reduce broadcast below.
				stream.send(CoConnectionOverview::from_state(&state.state, &co)).ok();
				state.co_overviews.entry(co).or_default().push(stream);
				return Ok(());
			},
		};

		// reduce
		let next_actions = state.state.reduce(action.clone());

		// handle internal actions (atomic within the handle call)
		for next_action in &next_actions {
			// we need to handle DidReleased atomic when its occured to not have a race condition with next actions
			if let ConnectionAction::DidReleased(released) = next_action {
				state.did_peers_changed.remove(&released.to);
			}
		}

		// response
		//  note: must be done after reducer to have use_initial return the correct results
		match response {
			Some(ResponseKind::Co(co, mut response)) => {
				if let Some(initial) = state.state.use_initial(&co) {
					response.send(initial).ok();
				}
				state.peers_changed.entry(co).or_default().push(response);
			},
			Some(ResponseKind::Did(did, mut response)) => {
				if let Some(initial) = state.state.did_use_initial(&did) {
					response.send(initial).ok();
				}
				state.did_peers_changed.entry(did).or_default().push(response);
			},
			None => {},
		}

		// epic
		state
			.epic
			.handle(&self.context.tasks, handle, &action, &state.state, &self.context);

		// responses
		match &action {
			ConnectionAction::PeersChanged(peers_changed_action) => {
				if let Some(responses) = state.peers_changed.get_mut(&peers_changed_action.id) {
					responses.send(peers_changed_action.clone());
				}
			},
			ConnectionAction::Released(released_action) => {
				state.peers_changed.remove(&released_action.id);
			},
			ConnectionAction::DidPeersChanged(did_peers_action) => {
				if let Some(responses) = state.did_peers_changed.get_mut(&did_peers_action.to) {
					responses.send(did_peers_action.clone());
				}
			},
			ConnectionAction::DidRelease(release) => {
				if let Some(responses) = state.did_peers_changed.get_mut(&release.to) {
					responses.retain_open();
				}
				if !state.state.did.contains_key(&release.to)
					|| state.did_peers_changed.get(&release.to).is_some_and(ResponseStreams::is_empty)
				{
					state.did_peers_changed.remove(&release.to);
				}
			},
			_ => {},
		}

		// overview snapshot stream
		if !state.overviews.is_empty() {
			state.overviews.send(ConnectionOverview::from(&state.state));
		}

		// CO overview streams
		state.co_overviews.retain(|co, streams| {
			if streams.is_empty() {
				return false;
			}
			streams.send(CoConnectionOverview::from_state(&state.state, co));
			!streams.is_empty()
		});

		// dispatch
		for next_action in next_actions {
			handle.dispatch(next_action)?;
		}

		// result
		Ok(())
	}
}

enum ResponseKind {
	Co(CoId, ResponseStream<PeersChangedAction>),
	Did(Did, ResponseStream<DidPeersChangedAction>),
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		connections::{DidReleasedAction, DidUseAction, DisconnectReason, DisconnectedAction},
		services::{
			connections::resolve::{NetworkResolver, StaticNetworkResolver},
			discovery::DiscoveryApi,
			network::CoNetworkTaskSpawner,
		},
	};
	use co_actor::{time::Instant, ResponseStreamReceiver};
	use co_identity::{
		IdentityResolver, MemoryIdentityResolver, MemoryPrivateIdentityResolver, PrivateIdentityResolver,
	};
	use co_primitives::{CoDate, Network, NetworkRendezvous, StaticCoDate};
	use futures::StreamExt;
	use libp2p::PeerId;
	use std::time::Duration;

	async fn initialized_connections() -> (Connections, State, ActorHandle<ConnectionMessage>) {
		let local_peer = PeerId::random();
		let connections = Connections::new(ConnectionsContext {
			date: StaticCoDate(0).boxed(),
			tasks: TaskSpawner::default(),
			settings: NetworkSettings::default(),
			network: CoNetworkTaskSpawner::new_closed(local_peer),
			identity_resolver: MemoryIdentityResolver::default().boxed(),
			private_identity_resolver: MemoryPrivateIdentityResolver::default().boxed(),
			network_resolver: StaticNetworkResolver::default().boxed(),
			discovery: DiscoveryApi::new_closed(),
		});
		let handle = ActorHandle::new_closed();
		let state = connections.initialize(&handle, &Tags::default(), ()).await.unwrap();
		(connections, state, handle)
	}

	fn did_use(from: &Did, to: &Did, network: &Network) -> DidUseAction {
		DidUseAction::new(from.clone(), to.clone(), Instant::now(), [network.clone()].into())
	}

	#[tokio::test]
	async fn failure_closes_the_failed_did_response_group_in_the_same_turn() {
		let (connections, mut state, handle) = initialized_connections().await;
		let from = Did::from("did:local:alice");
		let to = Did::from("did:local:bob");
		let network =
			Network::Rendezvous(NetworkRendezvous { namespace: "failed-response".to_owned(), addresses: vec![] });
		let (response, mut receiver) = ResponseStreamReceiver::new();

		connections
			.handle(&handle, ConnectionMessage::DidUse(did_use(&from, &to, &network), response), &mut state)
			.await
			.unwrap_err();
		connections
			.handle(
				&handle,
				DisconnectedAction { network, reason: DisconnectReason::Failure("route failed".to_owned()) }.into(),
				&mut state,
			)
			.await
			.unwrap_err();

		let closed = co_actor::time::timeout(Duration::from_secs(1), receiver.next())
			.await
			.expect("failed DID response group stayed open");
		assert!(closed.is_none());
	}

	#[tokio::test]
	async fn stale_did_released_notification_preserves_a_new_response_group() {
		let (connections, mut state, handle) = initialized_connections().await;
		let from = Did::from("did:local:alice");
		let to = Did::from("did:local:bob");
		let network =
			Network::Rendezvous(NetworkRendezvous { namespace: "new-response".to_owned(), addresses: vec![] });
		let action = did_use(&from, &to, &network);
		state.state.reduce(action.into());
		let (response, mut receiver) = ResponseStreamReceiver::new();
		state.did_peers_changed.entry(to.clone()).or_default().push(response);

		connections
			.handle(&handle, DidReleasedAction { to: to.clone() }.into(), &mut state)
			.await
			.unwrap();

		assert!(state.state.did.contains_key(&to));
		assert!(state.state.networks[&network].did_references.contains(&to));
		assert!(co_actor::time::timeout(Duration::from_millis(10), receiver.next())
			.await
			.is_err());
	}
}
