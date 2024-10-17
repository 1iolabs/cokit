use super::{ConnectionAction, ConnectionState};
use crate::{
	actor::{Epic, EpicExt, TracingEpic},
	CoContext,
};
use co_primitives::Tags;

mod connect;
mod disconnect;
mod network_resolve;

pub fn epic(tags: Tags) -> impl Epic<ConnectionAction, ConnectionState, CoContext> {
	connect::ConnectEpic::new()
		.join(network_resolve::NetworkResolveEpic::new())
		.join(disconnect::DisconnectEpic::new())
		.join(TracingEpic::new(tags))
}
