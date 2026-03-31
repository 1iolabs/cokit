// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{
	action::{DidSubscribeAction, DidUnsubscribeAction, DiscoveryAction, ReleaseAction},
	actor::DiscoveryActor,
	message::DiscoveryMessage,
	state::{did_discovery_subscription_topic_str, DidDiscoverySubscription},
};
use crate::services::discovery;
use co_actor::{ActorError, ActorHandle, ActorInstance};
use co_identity::network_did_discovery;
use co_primitives::Did;
use futures::Stream;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct DiscoveryApi {
	handle: ActorHandle<DiscoveryMessage>,
}
impl From<&ActorInstance<DiscoveryActor>> for DiscoveryApi {
	fn from(value: &ActorInstance<DiscoveryActor>) -> Self {
		Self { handle: value.handle() }
	}
}
impl DiscoveryApi {
	/// Create a closed (disconnected) API handle useful for tests.
	#[cfg(test)]
	pub fn new_closed() -> Self {
		Self { handle: ActorHandle::new_closed() }
	}

	/// Connect peers using discovery. Returns a stream of discovery events.
	pub fn connect(
		&self,
		discovery: BTreeSet<discovery::Discovery>,
	) -> impl Stream<Item = Result<discovery::Event, ActorError>> {
		self.handle
			.clone()
			.stream(|response| DiscoveryMessage::Connect(discovery, response))
	}

	/// Release a discovery request.
	pub fn release(&self, id: u64) {
		self.handle.dispatch(DiscoveryAction::Release(ReleaseAction { id })).ok();
	}

	/// Subscribe identity for DID discovery.
	pub fn did_subscribe(
		&self,
		identity: Option<co_identity::PrivateIdentityBox>,
		network: Option<co_primitives::NetworkDidDiscovery>,
	) -> Result<(), anyhow::Error> {
		let subscription = match identity {
			Some(identity) => {
				let network = network_did_discovery(&identity, network)?;
				DidDiscoverySubscription::Identity(network, identity)
			},
			None => DidDiscoverySubscription::Default,
		};
		let topic_str = did_discovery_subscription_topic_str(&subscription).to_owned();
		self.handle
			.dispatch(DiscoveryAction::DidSubscribe(DidSubscribeAction { subscription, topic_str }))?;
		Ok(())
	}

	/// Unsubscribe identity from DID discovery.
	pub fn did_unsubscribe(&self, did: Option<Did>) -> Result<(), anyhow::Error> {
		let action = match did {
			Some(did) => DidUnsubscribeAction::Identity(did),
			None => DidUnsubscribeAction::Default,
		};
		self.handle.dispatch(DiscoveryAction::DidUnsubscribe(action))?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		bitswap::BitswapMessage,
		connections::NetworkResolver,
		services::{
			discovery::{self, DidDiscovery, DidDiscoveryMessageType, DiscoverMessage, Discovery},
			network::{Network, NetworkApi, NetworkInitialize, NetworkMessage, NetworkSettings},
		},
	};
	use co_actor::{Actor, ActorHandle, TaskSpawner};
	use co_identity::{
		DidKeyIdentity, DidKeyIdentityResolver, IdentityResolver, MemoryPrivateIdentityResolver, PrivateIdentityBox,
		PrivateIdentityResolver,
	};
	use co_primitives::{tags, CoDate, NetworkPeer, StaticCoDate};
	use futures::StreamExt;
	use libp2p::identity::Keypair;
	use std::{collections::BTreeSet, time::Duration};

	#[derive(Debug, Default)]
	struct EmptyNetworkResolver;
	#[async_trait::async_trait]
	impl NetworkResolver for EmptyNetworkResolver {
		async fn networks(&self, _co: co_primitives::CoId) -> Result<BTreeSet<co_primitives::Network>, anyhow::Error> {
			Ok(Default::default())
		}
	}

	async fn spawn_peer(name: &str, identities: Vec<PrivateIdentityBox>) -> (ActorHandle<NetworkMessage>, NetworkApi) {
		let keypair = Keypair::generate_ed25519();
		let tasks = TaskSpawner::default();
		let settings = NetworkSettings::default()
			.with_localhost()
			.without_bootstrap()
			.with_mdns(false)
			.with_nat(false);
		let identity_resolver = DidKeyIdentityResolver::new().boxed();
		let private_identity_resolver = MemoryPrivateIdentityResolver::from(identities).boxed();
		let bitswap: ActorHandle<BitswapMessage> = ActorHandle::new_closed();
		let network = Actor::spawn_with(
			tasks.clone(),
			tags!("type": "network", "application": name),
			Network,
			NetworkInitialize {
				settings,
				identifier: name.to_owned(),
				keypair,
				date: StaticCoDate(0).boxed(),
				identity_resolver,
				private_identity_resolver,
				bitswap,
				tasks,
				network_resolver: EmptyNetworkResolver.boxed(),
			},
		)
		.unwrap();
		let handle = network.initialized().await.unwrap();
		let api = handle.request(NetworkMessage::Network).await.unwrap();
		(handle, api)
	}

	#[tokio::test]
	async fn test_peer_discovery() {
		let (_h1, network1) = spawn_peer("peer1", vec![]).await;
		let (_h2, network2) = spawn_peer("peer2", vec![]).await;

		// get peer1 listener address
		let peer1_id = network1.local_peer_id();
		let peer1_addrs = network1.listeners(true, false).await.unwrap();
		let peer1_addr = peer1_addrs.into_iter().next().unwrap();

		// peer2: discover peer1 using Peer discovery
		let discovery_set: BTreeSet<Discovery> =
			[Discovery::Peer(NetworkPeer { peer: peer1_id.to_bytes(), addresses: vec![peer1_addr.to_string()] })]
				.into_iter()
				.collect();

		let events = network2.discovery().connect(discovery_set);
		tokio::pin!(events);

		let event = tokio::time::timeout(Duration::from_secs(10), events.next())
			.await
			.expect("timeout waiting for discovery event")
			.expect("stream ended")
			.expect("actor error");

		match event {
			discovery::Event::Connected { peer, .. } => {
				assert_eq!(peer, peer1_id);
			},
			other => panic!("expected Connected event, got {:?}", other),
		}
	}

	#[tokio::test]
	async fn test_did_discovery() {
		// identities
		let did1 = DidKeyIdentity::generate(Some(&[1; 32]));
		let did2 = DidKeyIdentity::generate(Some(&[2; 32]));

		let (_h1, network1) = spawn_peer("peer1", vec![PrivateIdentityBox::new(did1.clone())]).await;
		let (_h2, network2) = spawn_peer("peer2", vec![PrivateIdentityBox::new(did2.clone())]).await;

		// get addresses
		let peer1_addrs: Vec<_> = network1.listeners(true, false).await.unwrap().into_iter().collect();
		let peer2_addrs: Vec<_> = network2.listeners(true, false).await.unwrap().into_iter().collect();
		let peer1_id = network1.local_peer_id();
		let peer2_id = network2.local_peer_id();

		// dial peer1 from peer2 so gossipsub can work
		network2.dial(Some(peer1_id), peer1_addrs.clone()).await.unwrap();

		// subscribe both peers for DID discovery
		network1
			.didcontact_subscribe(did1.clone(), co_identity::network_did_discovery(&did1, None).unwrap())
			.unwrap();
		network2
			.didcontact_subscribe(did2.clone(), co_identity::network_did_discovery(&did2, None).unwrap())
			.unwrap();

		// small delay for gossip subscriptions to propagate
		tokio::time::sleep(Duration::from_millis(500)).await;

		// peer2: connect via DID discovery targeting did1
		let did_discovery = DidDiscovery::create(
			&StaticCoDate(0),
			peer2_id,
			&did2,
			&did1,
			None,
			DidDiscoveryMessageType::Discover.to_string(),
			Some(&DiscoverMessage { endpoints: peer2_addrs.into_iter().collect() }),
		)
		.unwrap();

		let discovery_set: BTreeSet<Discovery> = [Discovery::DidDiscovery(did_discovery)].into_iter().collect();
		let events = network2.discovery().connect(discovery_set);
		tokio::pin!(events);

		let event = tokio::time::timeout(Duration::from_secs(10), events.next())
			.await
			.expect("timeout waiting for discovery event")
			.expect("stream ended")
			.expect("actor error");

		match event {
			discovery::Event::Connected { peer, .. } => {
				assert_eq!(peer, peer1_id);
			},
			other => panic!("expected Connected event, got {:?}", other),
		}
	}
}
