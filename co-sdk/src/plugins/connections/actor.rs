use super::{epics::connect::ConnectEpic, ConnectionAction, ConnectionMessage, ConnectionState};
use crate::{
	actor::{Actor, ActorError, ActorHandle, EpicRuntime, Reducer},
	CoContext,
};
use async_trait::async_trait;
use co_primitives::Tags;
use std::time::Duration;

// pub fn create_connections(tags: Tags, context: CoContext) -> Result<ActorHandle<ConnectionMessage>, ActorError> {
// 	Ok(Actor::spawn(
// 		tags,
// 		EpicActor::new(Connections { keep_alive: Duration::from_secs(30) }, || ConnectEpic {}, context),
// 		(),
// 	)?
// 	.handle())
// }

pub struct State {
	state: ConnectionState,
	epic: EpicRuntime<ConnectEpic, ConnectionMessage, ConnectionAction, ConnectionState, CoContext>,
}

pub struct Connections {
	context: CoContext,
	keep_alive: Duration,
}
#[async_trait]
impl Actor for Connections {
	type Message = ConnectionMessage;
	type State = State;
	type Initialize = ();

	async fn initialize(&self, _tags: Tags, _initialize: Self::Initialize) -> Result<Self::State, ActorError> {
		Ok(State {
			state: ConnectionState {
				keep_alive: self.keep_alive,
				cache: Default::default(),
				co: Default::default(),
				networks: Default::default(),
			},
			epic: EpicRuntime::new(ConnectEpic {}, |err| {
				tracing::error!(?err, "connection-epic-error");
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
		let action = match message {
			ConnectionMessage::Use(action, response) => Some(ConnectionAction::Use(action)),
			ConnectionMessage::Action(action) => Some(action),
			_ => None,
		};
		if let Some(action) = action {
			let actions = state.state.reduce(action.clone());

			// epic
			state.epic.handle(handle, &action, &state.state, &self.context);

			// dispatch
			for action in actions {
				handle.dispatch(action)?;
			}
		}

		// result
		Ok(())
	}
}

#[cfg(test)]
mod tests {

	#[tokio::test]
	async fn test_use() {}
}
