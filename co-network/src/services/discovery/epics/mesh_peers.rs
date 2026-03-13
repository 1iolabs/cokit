// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::{
	discovery::{
		action::{DiscoveryAction, MeshPeersResultAction, QueryMeshPeersAction},
		actor::DiscoveryContext,
		state::DiscoveryState,
	},
	network::MeshPeersNetworkTask,
};
use co_actor::Actions;
use futures::{FutureExt, Stream};

/// Handles `QueryMeshPeers` by querying mesh peers for a gossipsub topic.
pub fn mesh_peers_epic(
	_actions: &Actions<DiscoveryAction, DiscoveryState, DiscoveryContext>,
	action: &DiscoveryAction,
	_state: &DiscoveryState,
	context: &DiscoveryContext,
) -> Option<impl Stream<Item = Result<DiscoveryAction, anyhow::Error>> + Send + 'static> {
	match action {
		DiscoveryAction::QueryMeshPeers(QueryMeshPeersAction { request_id, topic }) => {
			let network = context.network.clone();
			let request_id = *request_id;
			let topic_hash = libp2p::gossipsub::TopicHash::from_raw(topic);
			Some(
				async move {
					let peers = MeshPeersNetworkTask::mesh_peers(&network, topic_hash).await?;
					Ok(DiscoveryAction::MeshPeersResult(MeshPeersResultAction { request_id, peers }))
				}
				.into_stream(),
			)
		},
		_ => None,
	}
}
