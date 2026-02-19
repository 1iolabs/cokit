// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{services::connections::state::BootstrapPeer, try_peer_id};
use multiaddr::{Multiaddr, PeerId};
use std::collections::{BTreeSet, HashMap};

/// Convert a list of [`Multiaddr`] to configures bootstrap peers.
pub fn bootstrap_from_multiaddrs(
	bootstrap: BTreeSet<Multiaddr>,
) -> Result<HashMap<PeerId, BootstrapPeer>, anyhow::Error> {
	bootstrap
		.into_iter()
		.map(|addr| -> Result<(PeerId, Multiaddr), anyhow::Error> { Ok((try_peer_id(&addr)?, addr)) })
		.try_fold(HashMap::default(), |mut result, item| {
			let (peer_id, addr) = item?;
			result
				.entry(peer_id)
				.or_insert_with_key(|peer_id| BootstrapPeer::new(*peer_id, Default::default()))
				.endpoints
				.insert(addr);
			Ok(result)
		})
}

#[cfg(test)]
mod tests {
	use crate::services::connections::library::bootstrap_from_multiaddrs::bootstrap_from_multiaddrs;
	use multiaddr::{multihash::Multihash, Multiaddr, PeerId};
	use std::{collections::BTreeSet, str::FromStr};

	#[test]
	fn test_bootstrap_from_multiaddrs() {
		let peer1 = PeerId::from_multihash(Multihash::wrap(0, &[0; 32]).unwrap()).unwrap();
		let peer2 = PeerId::from_str("12D3KooWEinh2zCgGbJaDfepoiiPiBgFcysSMYSc1EQrgEEZi9aX").unwrap();
		let bootstrap: BTreeSet<Multiaddr> = [
			format!("/ip4/127.0.0.1/tcp/9090/p2p/{}", peer1).parse().unwrap(),
			format!("/ip6/::1/tcp/9091/p2p/{}", peer1).parse().unwrap(),
			"/dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWEinh2zCgGbJaDfepoiiPiBgFcysSMYSc1EQrgEEZi9aX"
				.parse()
				.unwrap(),
		]
		.into_iter()
		.collect();

		let result = bootstrap_from_multiaddrs(bootstrap).unwrap();
		assert_eq!(result.len(), 2);
		assert_eq!(
			result.get(&peer1).unwrap().endpoints,
			[
				format!("/ip4/127.0.0.1/tcp/9090/p2p/{}", peer1).parse().unwrap(),
				format!("/ip6/::1/tcp/9091/p2p/{}", peer1).parse().unwrap()
			]
			.into_iter()
			.collect()
		);
		assert_eq!(
			result.get(&peer2).unwrap().endpoints,
			["/dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWEinh2zCgGbJaDfepoiiPiBgFcysSMYSc1EQrgEEZi9aX"
				.parse()
				.unwrap()]
			.into_iter()
			.collect()
		);
	}
}
