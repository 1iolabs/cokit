// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	backoff_with_jitter,
	connections::DialAction,
	services::connections::{
		action::ConnectionAction, actor::ConnectionsContext,
		library::find_connectable_bootstrap::find_connectable_bootstrap, ConnectionState,
	},
};
use co_actor::{time, Actions};
use futures::{stream, FutureExt, Stream, StreamExt};

/// Dial a bootstrap when no bootstrap peer is connected.
pub fn bootstrap_health_epic(
	_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
	message: &ConnectionAction,
	state: &ConnectionState,
	_context: &ConnectionsContext,
) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
	let ConnectionAction::PeerConnectionClosed(action) = message else {
		return None;
	};

	// only act when a bootstrap peer disconnected
	if !state.bootstrap.contains_key(&action.peer_id) {
		return None;
	}

	// check if any bootstrap is still connected
	let has_connected_bootstrap = state
		.bootstrap
		.keys()
		.any(|peer_id| state.peers.get(peer_id).map(|p| p.connected).unwrap_or(false));
	if has_connected_bootstrap {
		return None;
	}

	// no bootstrap connected — find one to dial
	let next_attempt = find_connectable_bootstrap(state, time::Instant::now(), backoff_with_jitter);
	Some(
		async move {
			let action = match next_attempt {
				Ok(bootstrap) => Some(ConnectionAction::Dial(DialAction {
					peer_id: bootstrap.peer_id,
					endpoints: bootstrap.endpoints.clone(),
				})),
				Err(Some(next_attempt)) => {
					time::sleep_until(next_attempt).await;
					Some(ConnectionAction::InsufficentPeers)
				},
				Err(None) => None,
			};
			action.into_iter()
		}
		.into_stream()
		.flat_map(|iter| stream::iter(iter).map(Ok)),
	)
}

#[cfg(test)]
mod tests {
	use crate::{
		connections::{ConnectionAction, ConnectionState, DialAction, NetworkResolver, PeerConnectionClosedAction},
		services::{
			connections::{
				epics::bootstrap_health::bootstrap_health_epic,
				resolve::StaticNetworkResolver,
				state::{BootstrapPeer, PeerConnection},
				ConnectionsContext,
			},
			network::CoNetworkTaskSpawner,
		},
		NetworkSettings,
	};
	use cid::multihash::Multihash;
	use co_actor::{time::Instant, Actions, TaskSpawner};
	use co_identity::{
		IdentityResolver, MemoryIdentityResolver, MemoryPrivateIdentityResolver, PrivateIdentityResolver,
	};
	use co_primitives::{CoDate, StaticCoDate};
	use futures::TryStreamExt;
	use libp2p::{Multiaddr, PeerId};
	use std::{
		collections::{BTreeSet, HashMap},
		str::FromStr,
		time::Duration,
	};
	use tokio::time::timeout;

	fn test_context() -> (Actions<ConnectionAction, ConnectionState, ConnectionsContext>, ConnectionsContext) {
		let local_peer = PeerId::from_multihash(Multihash::wrap(0, &[0; 32]).unwrap()).unwrap();
		let actions = Actions::default();
		let context = ConnectionsContext {
			date: StaticCoDate(0).boxed(),
			tasks: TaskSpawner::default(),
			settings: NetworkSettings::default(),
			network: CoNetworkTaskSpawner::new_closed(local_peer),
			identity_resolver: MemoryIdentityResolver::default().boxed(),
			private_identity_resolver: MemoryPrivateIdentityResolver::default().boxed(),
			network_resolver: StaticNetworkResolver::default().boxed(),
			discovery: crate::services::discovery::DiscoveryApi::new_closed(),
		};
		(actions, context)
	}

	fn bootstrap_peer(id: u8) -> (PeerId, BootstrapPeer, BTreeSet<Multiaddr>) {
		let peer_id = PeerId::from_multihash(Multihash::wrap(0, &[id; 32]).unwrap()).unwrap();
		let endpoints: BTreeSet<Multiaddr> = [Multiaddr::from_str("/dns4/bootstrap.1io.com/udp/5000/quic-v1").unwrap()]
			.into_iter()
			.collect();
		let bootstrap = BootstrapPeer::new(peer_id, endpoints.clone());
		(peer_id, bootstrap, endpoints)
	}

	#[tokio::test]
	async fn dials_bootstrap_when_last_bootstrap_disconnects() {
		let (actions, context) = test_context();
		let (peer1, bootstrap1, endpoints1) = bootstrap_peer(1);
		let mut bootstrap = HashMap::new();
		bootstrap.insert(peer1, bootstrap1);
		let state = ConnectionState { keep_alive: Duration::from_secs(30), bootstrap, ..Default::default() };

		let message =
			ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction { peer_id: peer1, time: Instant::now() });
		let stream = bootstrap_health_epic(&actions, &message, &state, &context).unwrap();
		let result = timeout(Duration::from_secs(1), stream.try_collect::<Vec<_>>())
			.await
			.unwrap()
			.unwrap();
		assert_eq!(result, vec![ConnectionAction::Dial(DialAction { peer_id: peer1, endpoints: endpoints1 })]);
	}

	#[tokio::test]
	async fn skips_when_other_bootstrap_still_connected() {
		let (actions, context) = test_context();
		let (peer1, bootstrap1, _) = bootstrap_peer(1);
		let (peer2, bootstrap2, _) = bootstrap_peer(2);
		let mut bootstrap = HashMap::new();
		bootstrap.insert(peer1, bootstrap1);
		bootstrap.insert(peer2, bootstrap2);

		let mut peers = HashMap::new();
		peers.insert(peer2, PeerConnection { connected: true, ..Default::default() });

		let state = ConnectionState { keep_alive: Duration::from_secs(30), bootstrap, peers, ..Default::default() };

		let message =
			ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction { peer_id: peer1, time: Instant::now() });
		let result = bootstrap_health_epic(&actions, &message, &state, &context);
		assert!(result.is_none());
	}

	#[tokio::test]
	async fn ignores_non_bootstrap_peer_close() {
		let (actions, context) = test_context();
		let (peer1, bootstrap1, _) = bootstrap_peer(1);
		let non_bootstrap = PeerId::from_multihash(Multihash::wrap(0, &[99; 32]).unwrap()).unwrap();
		let mut bootstrap = HashMap::new();
		bootstrap.insert(peer1, bootstrap1);
		let state = ConnectionState { keep_alive: Duration::from_secs(30), bootstrap, ..Default::default() };

		let message = ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction {
			peer_id: non_bootstrap,
			time: Instant::now(),
		});
		let result = bootstrap_health_epic(&actions, &message, &state, &context);
		assert!(result.is_none());
	}
}
