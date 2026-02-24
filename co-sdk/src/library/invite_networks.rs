// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoContext;
use co_network::identities_networks;
use co_primitives::{CoInviteMetadata, Network, NetworkPeer};
use futures::TryStreamExt;
use std::collections::BTreeSet;

/// Get Network instances from invite metadata.
pub async fn invite_networks(
	context: &CoContext,
	invite: &CoInviteMetadata,
) -> Result<BTreeSet<Network>, anyhow::Error> {
	// network settings
	let mut networks = if !invite.network.network.is_empty() {
		invite.network.network.clone()
	} else {
		// participants
		let identity_resolver = context.identity_resolver().await?;
		identities_networks(Some(&identity_resolver), invite.network.participants.iter().cloned())
			.try_collect()
			.await?
	};

	// the invite peer (maybe still connected)
	if let Some(peer) = &invite.peer {
		networks.insert(Network::Peer(NetworkPeer { peer: peer.clone(), addresses: vec![] }));
	}

	Ok(networks)
}
