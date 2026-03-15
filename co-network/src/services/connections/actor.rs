// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{
	action::{ConnectionAction, DidPeersChangedAction, PeersChangedAction},
	epics::epic,
	ConnectionMessage, ConnectionState,
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
use std::collections::BTreeMap;

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
		};

		// reduce
		let next_actions = state.state.reduce(action.clone());

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
			ConnectionAction::DidReleased(released_did_action) => {
				state.did_peers_changed.remove(&released_did_action.to);
			},
			_ => {},
		}

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
