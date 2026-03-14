// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{action::ConnectionAction, ConnectionState};
use crate::services::connections::actor::ConnectionsContext;
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

mod bootstrap_health;
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
		.join(bootstrap_health::bootstrap_health_epic)
		.join(TracingEpic::new(tags))
}
