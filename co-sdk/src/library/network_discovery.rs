use co_identity::{network_did_discovery, Identity, IdentityResolver, IdentityResolverBox, PrivateIdentity};
use co_network::{discovery, heads};
use co_primitives::{CoId, Did, Network};
use futures::{stream::iter, Stream};
use std::collections::BTreeSet;
use tokio_stream::StreamExt;

/// Create Discovery items from networks and participants.
///
/// Errors are retunred with the stream befor continue with the other items.
pub fn network_discovery<'a, P>(
	identity_resolver: Option<&'a IdentityResolverBox>,
	from: &'a P,
	id: Option<&'a CoId>,
	networks: impl IntoIterator<Item = Network> + 'a,
	identities: impl IntoIterator<Item = Did> + 'a,
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
					for await discovery_result in network_discovery_one(identity_resolver, from, id, network) {
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

fn identities_networks<'a>(
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

fn network_discovery_one<'a, P>(
	identity_resolver: Option<&'a IdentityResolverBox>,
	from: &'a P,
	id: Option<&'a CoId>,
	network: Network,
) -> impl Stream<Item = Result<discovery::Discovery, anyhow::Error>> + 'a
where
	P: PrivateIdentity + Send + Sync + 'static,
{
	async_stream::stream! {
		match network {
			Network::CoHeads(value) =>
			{
				if let Some(id) = &id {
					yield Ok(discovery::Discovery::Topic(heads::HeadsState::to_topic_hash(&value, id).into_string()));
				}
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
					yield discovery::DidDiscovery::create(
						from,
						&identity,
						Some(value),
						discovery::DidDiscoveryMessage::Discover.to_string(),
					)
						.map(|item| discovery::Discovery::DidDiscovery(item));
				}
			},
			_ => {},
		};
	}
}
