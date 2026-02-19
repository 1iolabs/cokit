// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
	if network.did != identity.identity() {
		return Err(anyhow::anyhow!("Invalid arguments"));
	}
	Ok(network)
}
