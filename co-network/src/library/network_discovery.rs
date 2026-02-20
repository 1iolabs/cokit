use crate::{discovery, services::heads::HeadsApi};
use co_identity::{network_did_discovery, Identity, IdentityResolver, IdentityResolverBox, PrivateIdentity};
use co_primitives::{Did, DynamicCoDate, Network};
use futures::{stream::iter, Stream};
use libp2p::{Multiaddr, PeerId};
use std::collections::BTreeSet;
use tokio_stream::StreamExt;

/// Create Discovery items from networks and participants.
/// Errors are returned with the stream before continue with the other items.
///
/// # Arguments
/// - `endpoints` - Our local endpoints others can use to dial us.
pub fn network_discovery<'a, P>(
	date: DynamicCoDate,
	identity_resolver: Option<&'a IdentityResolverBox>,
	from_peer: PeerId,
	from: &'a P,
	networks: impl IntoIterator<Item = Network> + 'a,
	identities: impl IntoIterator<Item = Did> + 'a,
	endpoints: BTreeSet<Multiaddr>,
) -> impl Stream<Item = Result<discovery::Discovery, anyhow::Error>> + 'a
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	async_stream::stream! {
		let mut seen = BTreeSet::new();

		// networks
		for await network in iter(networks.into_iter().map(Ok)).merge(identities_networks(identity_resolver, identities)) {
			match network {
				Ok(network) => {
					for await discovery_result in network_discovery_one(date.clone(), identity_resolver, from_peer, from, network, &endpoints) {
						match discovery_result {
							Ok(discovery) => {
								if seen.insert(discovery.clone()) {
									yield Ok(discovery);
								}
							},
							Err(err) => {
								yield Err(err);
							},
						}
					}
				}
				Err(err) => {
					yield Err(err);
				},
			}
		}
	}
}

/// Resolve networks from identities.
pub fn identities_networks<'a>(
	identity_resolver: Option<&'a IdentityResolverBox>,
	identities: impl IntoIterator<Item = Did> + 'a,
) -> impl Stream<Item = Result<Network, anyhow::Error>> + 'a {
	async_stream::stream! {
		if let Some(identity_resolver) = &identity_resolver {
			for did in identities {
				let identity = match identity_resolver.resolve(&did).await {
					Ok(identity) => identity,
					Err(err) => {
						yield Err(err.into());
						continue;
					},
				};
				let mut networks = identity.networks();
				if networks.is_empty() {
					networks.insert(Network::DidDiscovery(network_did_discovery(&identity, None)?));
				}
				for network in networks {
					yield Ok(network);
				}
			}
		}
	}
}

fn network_discovery_one<'a, 'b, P>(
	date: DynamicCoDate,
	identity_resolver: Option<&'a IdentityResolverBox>,
	from_peer: PeerId,
	from: &'a P,
	network: Network,
	endpoints: &'b BTreeSet<Multiaddr>,
) -> impl Stream<Item = Result<discovery::Discovery, anyhow::Error>> + use<'a, 'b, P>
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	async_stream::stream! {
		let mut body = None;
		match network {
			Network::CoHeads(value) =>
			{
				yield Ok(discovery::Discovery::Topic(HeadsApi::to_topic_hash(&value).into_string()));
			},
			Network::Rendezvous(value) => {
				yield Ok(discovery::Discovery::Rendezvous(value));
			},
			Network::Peer(value) => {
				yield Ok(discovery::Discovery::Peer(value));
			},
			Network::DidDiscovery(value) => {
				if let Some(identity_resolver) = &identity_resolver {
					let identity = identity_resolver.resolve(&value.did).await?;
					if body.is_none() {
						body = Some(discovery::DiscoverMessage {
							endpoints: endpoints.clone(),
						});
					}
					yield discovery::DidDiscovery::create(
						&date,
						from_peer,
						from,
						&identity,
						Some(value),
						discovery::DidDiscoveryMessageType::Discover.to_string(),
						body.as_ref(),
					)
						.map(discovery::Discovery::DidDiscovery);
				}
			},
			_ => {},
		};
	}
}
