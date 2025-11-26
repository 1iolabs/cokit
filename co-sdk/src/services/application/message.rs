use super::Action;
use crate::CoContext;
use co_actor::{Response, ResponseStream};
use co_network::NetworkApi;
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
	Network(Response<Result<NetworkApi, anyhow::Error>>),
}
impl From<Action> for ApplicationMessage {
	fn from(value: Action) -> Self {
		ApplicationMessage::Dispatch(value)
	}
}
