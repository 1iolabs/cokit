use super::{epics::epic, ConnectionAction, ConnectionMessage, ConnectionState, PeersChangedAction};
use crate::CoContext;
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, Reducer, ResponseStreams};
use co_primitives::{CoId, Tags};
use std::{collections::BTreeMap, time::Duration};

pub struct State {
	state: ConnectionState,
	epic: EpicRuntime<ConnectionMessage, ConnectionAction, ConnectionState, CoContext>,
	peers_changed: BTreeMap<CoId, ResponseStreams<PeersChangedAction>>,
}

pub struct Connections {
	context: CoContext,
	keep_alive: Duration,
}
impl Connections {
	pub fn new(context: CoContext, keep_alive: Duration) -> Self {
		Self { context, keep_alive }
	}
}
#[async_trait]
impl Actor for Connections {
	type Message = ConnectionMessage;
	type State = State;
	type Initialize = ();

	async fn initialize(&self, tags: Tags, _initialize: Self::Initialize) -> Result<Self::State, ActorError> {
		Ok(State {
			state: ConnectionState {
				keep_alive: self.keep_alive,
				_cache: Default::default(),
				co: Default::default(),
				networks: Default::default(),
			},
			epic: EpicRuntime::new(epic(tags), |err| {
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
		let action = match message {
			ConnectionMessage::Use(action, response) => {
				// add response
				state
					.peers_changed
					.entry(action.id.clone())
					.or_insert(Default::default())
					.push(response);

				// action
				Some(ConnectionAction::Use(action))
			},
			ConnectionMessage::Action(action) => Some(action),
		};
		if let Some(action) = action {
			let next_actions = state.state.reduce(action.clone());

			// epic
			state.epic.handle(handle, &action, &state.state, &self.context);

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
		}

		// result
		Ok(())
	}
}
