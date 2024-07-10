use co_identity::{Identity, PrivateIdentity};
use co_network::{
	discovery::{self, Discovery},
	DidDiscoveryMessage,
};
use co_primitives::Network;
use std::collections::BTreeSet;

/// Create discovery items from identity networks.
pub fn identity_discovery<P, I>(from: &P, to: &I) -> Result<BTreeSet<Discovery>, anyhow::Error>
where
	P: PrivateIdentity + Send + Sync + 'static,
	I: Identity + Send + Sync + 'static,
{
	let mut networks = to.networks();
	if networks.is_empty() {
		networks.insert(Network::DidDiscovery(Default::default()));
	}
	Ok(to
		.networks()
		.into_iter()
		.filter_map(|network| match network {
			Network::DidDiscovery(value) => Some(Discovery::DidDiscovery(
				discovery::DidDiscovery::create(from, to, value, DidDiscoveryMessage::Discover.to_string()).ok()?,
			)),
			Network::Rendezvous(value) => Some(Discovery::Rendezvous(value)),
			Network::Peer(value) => Some(Discovery::Peer(value)),
			_ => None,
		})
		.collect())
}
