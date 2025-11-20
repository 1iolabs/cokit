use super::{action::ConnectionAction, ConnectionState};
use crate::services::connections::actor::ConnectionsContext;
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

mod connect;
mod dial;
mod disconnect;
mod insufficent_peers;
mod network_resolve;
mod peers_threshold;

pub fn epic(tags: Tags) -> impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> {
	MergeEpic::new()
		.join(connect::ConnectEpic::new())
		.join(network_resolve::NetworkResolveEpic::new())
		.join(disconnect::DisconnectEpic::new())
		.join(dial::dial_epic)
		.join(insufficent_peers::InsufficentPeersEpic::default())
		.join(peers_threshold::peers_threshold_epic)
		.join(TracingEpic::new(tags))
}
