use crate::Identity;
use co_primitives::NetworkDidDiscovery;

/// Create `NetworkDidDiscovery` from identity or configuration.
pub fn network_did_discovery<I>(
	identity: &I,
	network: Option<NetworkDidDiscovery>,
) -> Result<NetworkDidDiscovery, anyhow::Error>
where
	I: Identity,
{
	let network = network
		.unwrap_or_else(|| NetworkDidDiscovery { did: identity.identity().to_owned(), topic: Default::default() });
	if &network.did != identity.identity() {
		return Err(anyhow::anyhow!("Invalid arguments"));
	}
	Ok(network)
}
