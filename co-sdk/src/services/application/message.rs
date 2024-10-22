use super::Action;
use crate::CoContext;
use co_actor::{Response, ResponseStream};

#[derive(Debug)]
pub enum ApplicationMessage {
	Dispatch(Action),
	Subscribe(ResponseStream<Action>),
	Context(Response<CoContext>),
}
impl From<Action> for ApplicationMessage {
	fn from(value: Action) -> Self {
		ApplicationMessage::Dispatch(value)
	}
}
