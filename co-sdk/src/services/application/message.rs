use super::Action;
use crate::{CoContext, NetworkMessage};
use co_actor::{ActorHandle, Response, ResponseStream};
use std::fmt::Debug;

#[derive(Debug)]
pub enum ApplicationMessage {
	/// Dispatch action.
	Dispatch(Action),

	/// Subscribe to actions.
	Subscribe(ResponseStream<Action>),

	// Get Context.
	Context(Response<CoContext>),

	/// Get Network.
	Network(Response<Result<ActorHandle<NetworkMessage>, anyhow::Error>>),
}
impl From<Action> for ApplicationMessage {
	fn from(value: Action) -> Self {
		ApplicationMessage::Dispatch(value)
	}
}
