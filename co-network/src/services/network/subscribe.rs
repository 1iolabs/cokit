// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
