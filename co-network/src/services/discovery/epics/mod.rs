// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{action::DiscoveryAction, actor::DiscoveryContext, state::DiscoveryState};
use co_actor::{Epic, MergeEpic, TracingEpic};
use co_primitives::Tags;

mod connect;
mod did_listen;
mod did_publish;
mod did_subscribe;
mod did_unsubscribe;
mod mesh_peers;
mod timeout;

pub fn epic(tags: Tags) -> impl Epic<DiscoveryAction, DiscoveryState, DiscoveryContext> {
	MergeEpic::new()
		.join(connect::dial_epic)
		.join(connect::send_resolve_epic)
		.join(did_subscribe::did_subscribe_epic)
		.join(did_unsubscribe::did_unsubscribe_epic)
		.join(did_publish::did_publish_epic)
		.join(did_listen::DidListenEpic::new())
		.join(mesh_peers::mesh_peers_epic)
		.join(timeout::TimeoutEpic::new())
		.join(TracingEpic::new(tags))
}
