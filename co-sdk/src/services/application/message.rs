// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::Action;
use crate::CoContext;
use co_actor::{Response, ResponseStream};
use co_network::NetworkApi;
use std::fmt::Debug;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
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
