use co_identity::{network_did_discovery, Identity, PrivateIdentity};
use co_network::discovery;
use co_primitives::Network;
use std::collections::BTreeSet;

/// Create discovery items from identity networks.
pub fn identity_discovery<P, I>(from: &P, to: &I) -> Result<BTreeSet<discovery::Discovery>, anyhow::Error>
where
	P: PrivateIdentity + Send + Sync + 'static,
	I: Identity + Send + Sync + 'static,
{
	let mut networks = to.networks();
	if networks.is_empty() {
		networks.insert(Network::DidDiscovery(network_did_discovery(to, None)?));
	}
	Ok(to
		.networks()
		.into_iter()
		.filter_map(|network| match network {
			Network::DidDiscovery(value) => Some(discovery::Discovery::DidDiscovery(
				discovery::DidDiscovery::create(
					from,
					to,
					Some(value),
					discovery::DidDiscoveryMessage::Discover.to_string(),
				)
				.ok()?,
			)),
			Network::Rendezvous(value) => Some(discovery::Discovery::Rendezvous(value)),
			Network::Peer(value) => Some(discovery::Discovery::Peer(value)),
			_ => None,
		})
		.collect())
}
