use super::{
	action::{ConnectionAction, PeersChangedAction},
	epics::epic,
	ConnectionMessage, ConnectionState,
};
use crate::{
	services::{
		connections::{library::bootstrap_from_multiaddrs::bootstrap_from_multiaddrs, resolve::DynamicNetworkResolver},
		network::CoNetworkTaskSpawner,
	},
	NetworkSettings,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, Reducer, ResponseStreams, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use co_primitives::{CoId, Tags};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ConnectionsContext {
	pub tasks: TaskSpawner,
	pub settings: NetworkSettings,
	pub network: CoNetworkTaskSpawner,
	pub identity_resolver: IdentityResolverBox,
	pub private_identity_resolver: PrivateIdentityResolverBox,
	pub network_resolver: DynamicNetworkResolver,
}

pub struct State {
	state: ConnectionState,
	epic: EpicRuntime<ConnectionMessage, ConnectionAction, ConnectionState, ConnectionsContext>,
	peers_changed: BTreeMap<CoId, ResponseStreams<PeersChangedAction>>,
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
				networks: Default::default(),
				peers: Default::default(),
				bootstrap: bootstrap_from_multiaddrs(self.context.settings.bootstrap.clone())?,
			},
			epic: EpicRuntime::new(epic(tags.clone()), |err| {
				tracing::error!(?err, "connection-epic-error");
				None
			}),
			peers_changed: Default::default(),
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
				(ConnectionAction::Use(action), Some((co, response)))
			},
			ConnectionMessage::Action(action) => (action, None),
		};

		// reduce
		let next_actions = state.state.reduce(action.clone());

		// response
		//  note: must be done after reducer to have use_initial return the correct results
		if let Some((co, mut response)) = response {
			// initial
			if let Some(initial) = state.state.use_initial(&co) {
				response.send(initial).ok();
			}

			// add response
			state.peers_changed.entry(co).or_insert(Default::default()).push(response);
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
