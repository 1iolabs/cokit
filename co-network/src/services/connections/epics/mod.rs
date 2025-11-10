use super::{action::ConnectionAction, ConnectionState};
use crate::services::connections::actor::ConnectionsContext;
use co_actor::{Epic, EpicExt, TracingEpic};
use co_primitives::Tags;

mod connect;
mod disconnect;
mod network_resolve;

pub fn epic(tags: Tags) -> impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> {
	connect::ConnectEpic::new()
		.join(network_resolve::NetworkResolveEpic::new())
		.join(disconnect::DisconnectEpic::new())
		.join(TracingEpic::new(tags))
}
