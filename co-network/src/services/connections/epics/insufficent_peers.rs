// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	backoff_with_jitter,
	connections::DialAction,
	services::connections::{
		action::ConnectionAction, actor::ConnectionsContext,
		library::find_connectable_bootstrap::find_connectable_bootstrap, ConnectionState,
	},
};
use co_actor::{Actions, Epic};
use futures::{stream, FutureExt, Stream, StreamExt};
use std::time::Instant;

/// Dial a bootstrap when we have a insufficent peers condition.
#[derive(Debug, Default)]
pub struct InsufficentPeersEpic {}
impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> for InsufficentPeersEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
		message: &ConnectionAction,
		state: &ConnectionState,
		_context: &ConnectionsContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::InsufficentPeers => {
				let next_attempt = find_connectable_bootstrap(state, Instant::now(), backoff_with_jitter);
				Some(
					async move {
						let action = match next_attempt {
							Ok(bootstrap) => Some(ConnectionAction::Dial(DialAction {
								peer_id: bootstrap.peer_id,
								endpoints: bootstrap.endpoints.clone(),
							})),
							Err(Some(next_attempt)) => {
								tokio::time::sleep_until(next_attempt.into()).await;
								Some(ConnectionAction::InsufficentPeers)
							},
							Err(None) => None,
						};
						action.into_iter()
					}
					.into_stream()
					.flat_map(|iter| stream::iter(iter).map(Ok)),
				)
			},
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		connections::{ConnectionAction, ConnectionState, DialAction, NetworkResolver},
		services::{
			connections::{
				epics::insufficent_peers::InsufficentPeersEpic, resolve::StaticNetworkResolver, state::BootstrapPeer,
				ConnectionsContext,
			},
			network::CoNetworkTaskSpawner,
		},
		NetworkSettings,
	};
	use cid::multihash::Multihash;
	use co_actor::{Actions, Epic, TaskSpawner};
	use co_identity::{
		IdentityResolver, MemoryIdentityResolver, MemoryPrivateIdentityResolver, PrivateIdentityResolver,
	};
	use futures::TryStreamExt;
	use libp2p::{Multiaddr, PeerId};
	use std::{
		collections::{BTreeSet, HashMap},
		str::FromStr,
		time::Duration,
	};
	use tokio::time::timeout;

	#[tokio::test]
	async fn test_insufficent_peers() {
		let actions = Actions::default();
		let mut bootstrap = HashMap::new();
		let local_peer = PeerId::from_multihash(Multihash::wrap(0, &[0; 32]).unwrap()).unwrap();
		let bootstrap_peer1 = PeerId::from_multihash(Multihash::wrap(0, &[1; 32]).unwrap()).unwrap();
		let bootstrap_peer1_endpoints: BTreeSet<Multiaddr> =
			[Multiaddr::from_str("/dns4/bootstrap.1io.com/udp/5000/quic-v1").unwrap()]
				.into_iter()
				.collect();
		bootstrap.insert(bootstrap_peer1, BootstrapPeer::new(bootstrap_peer1, bootstrap_peer1_endpoints.clone()));
		let state = ConnectionState {
			keep_alive: Duration::from_secs(30),
			co: Default::default(),
			networks: Default::default(),
			peers: Default::default(),
			bootstrap,
		};
		let context = ConnectionsContext {
			tasks: TaskSpawner::default(),
			settings: NetworkSettings::default(),
			network: CoNetworkTaskSpawner::new_closed(local_peer),
			identity_resolver: MemoryIdentityResolver::default().boxed(),
			private_identity_resolver: MemoryPrivateIdentityResolver::default().boxed(),
			network_resolver: StaticNetworkResolver::default().boxed(),
		};
		let mut epic = InsufficentPeersEpic::default();
		let stream = epic
			.epic(&actions, &ConnectionAction::InsufficentPeers, &state, &context)
			.unwrap();
		let actions = timeout(Duration::from_secs(1), stream.try_collect::<Vec<ConnectionAction>>())
			.await
			.unwrap()
			.unwrap();
		assert_eq!(
			actions,
			vec![ConnectionAction::Dial(DialAction { peer_id: bootstrap_peer1, endpoints: bootstrap_peer1_endpoints })]
		);
	}
}
