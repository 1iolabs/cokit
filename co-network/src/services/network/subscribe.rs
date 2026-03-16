// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::services::network::NetworkApi;
use co_identity::PrivateIdentity;
use co_primitives::NetworkDidDiscovery;
use std::fmt::Debug;

/// Listen on identity requests (DID Discovery).
pub async fn subscribe_identity<I>(network: &NetworkApi, identity: &I) -> Result<(), anyhow::Error>
where
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
	// get did discovery networks
	let mut networks: Vec<_> = identity
		.networks()
		.into_iter()
		.filter_map(|network| match network {
			co_primitives::Network::DidDiscovery(item) => Some(item),
			_ => None,
		})
		.collect();
	if networks.is_empty() {
		networks.push(NetworkDidDiscovery { did: identity.identity().to_owned(), topic: Default::default() });
	}

	// subscribe
	//  by returning on any error happens in between
	for item in networks {
		network.didcontact_subscribe(identity.clone(), item)?;
	}

	// result
	Ok(())
}
