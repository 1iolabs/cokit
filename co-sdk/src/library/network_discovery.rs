use co_identity::{IdentityBox, PrivateIdentity};
use co_network::{discovery, heads};
use co_primitives::{CoId, Network};
use std::collections::BTreeSet;

/// Create Discovery items from networks and participants.
pub async fn network_discovery<P>(
	identity: &P,
	id: &CoId,
	networks: impl IntoIterator<Item = Network>,
	participants: impl IntoIterator<Item = IdentityBox>,
) -> Result<BTreeSet<discovery::Discovery>, anyhow::Error>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	let co_networks = networks.into_iter().filter_map(|network| match network {
		Network::CoHeads(value) => {
			Some(discovery::Discovery::Topic(heads::HeadsState::to_topic_hash(&value, id).into_string()))
		},
		Network::Rendezvous(value) => Some(discovery::Discovery::Rendezvous(value)),
		Network::Peer(value) => Some(discovery::Discovery::Peer(value)),
		_ => None,
	});
	let participant_networks = participants.into_iter().flat_map(|participant| {
		identity.networks().into_iter().filter_map(move |network| match network {
			Network::DidDiscovery(value) => Some(discovery::Discovery::DidDiscovery(
				discovery::DidDiscovery::create(
					identity,
					&participant,
					value,
					discovery::DidDiscoveryMessage::Discover.to_string(),
				)
				.ok()?,
			)),
			Network::Rendezvous(value) => Some(discovery::Discovery::Rendezvous(value)),
			Network::Peer(value) => Some(discovery::Discovery::Peer(value)),
			_ => None,
		})
	});
	Ok(co_networks.chain(participant_networks).collect())
}
