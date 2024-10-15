use super::{
	action::{ConnectAction, ConnectedAction, ConnectionAction, DisconnectedAction, UseAction},
	NetworkResolveAction, PeersChangedAction,
};
use crate::actor::Reducer;
use co_primitives::{CoId, Did, Network};
use libp2p::{Multiaddr, PeerId};
use std::{
	collections::{BTreeSet, HashMap},
	time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct CoConnection {
	pub id: CoId,
	pub from: Did,
	pub networks: BTreeSet<Network>,
	pub keep_alive: Instant,
}

#[derive(Debug, Clone)]
pub struct NetworkConnection {
	pub network: Network,
	pub references: BTreeSet<CoId>,
	pub peers: BTreeSet<PeerId>,
	pub keep_alive: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionState {
	pub keep_alive: Duration,
	pub co: HashMap<CoId, CoConnection>,
	pub networks: HashMap<Network, NetworkConnection>,
	pub cache: HashMap<Network, BTreeSet<Multiaddr>>,
}
impl Reducer<ConnectionAction> for ConnectionState {
	fn reduce(&mut self, action: ConnectionAction) -> Vec<ConnectionAction> {
		let state = self;
		let mut actions = vec![];

		// state
		match &action {
			// TODO: use did? create new connections when different to existing?
			ConnectionAction::Use(UseAction { id, from, time, networks }) => {
				let mut networks = networks.clone();

				// co connections
				match state.co.get_mut(id) {
					Some(co_connection) => {
						// clear networks already in use
						networks.retain(|network| !co_connection.networks.contains(network));

						// insert networks
						for network in networks.iter() {
							co_connection.networks.insert(network.clone());
						}
					},
					None => {
						// insert co
						state.co.insert(
							id.clone(),
							CoConnection {
								id: id.clone(),
								from: from.clone(),
								keep_alive: *time + state.keep_alive,
								networks: networks.clone(),
							},
						);

						// resolve networks if not specified
						if networks.is_empty() {
							actions.push(ConnectionAction::NetworkResolve(NetworkResolveAction { id: id.clone() }));
						}
					},
				}

				// network connections
				for network in networks.iter() {
					// networks: get/create
					match state.networks.get_mut(network) {
						Some(network_connection) => {
							// reference
							network_connection.references.insert(id.clone());
							network_connection.keep_alive = *time + state.keep_alive;
						},
						None => {
							// insert
							state.networks.insert(
								network.clone(),
								NetworkConnection {
									keep_alive: *time + state.keep_alive,
									network: network.clone(),
									peers: Default::default(),
									references: [id.clone()].into_iter().collect(),
								},
							);

							// connect
							actions.push(ConnectionAction::Connect(ConnectAction {
								network: network.clone(),
								from: from.clone(),
							}));
						},
					};
				}

				// initial
				let intital_peers = state.co_peers(id);
				if !intital_peers.is_empty() {
					actions.push(ConnectionAction::PeersChanged(PeersChangedAction {
						id: id.clone(),
						peers: intital_peers.clone(),
						added: intital_peers,
						removed: Default::default(),
					}));
				}
			},
			ConnectionAction::Connected(ConnectedAction { network, result }) => {
				if let Some(network) = state.networks.get_mut(network) {
					match result {
						Ok(peers) => {
							// diff
							let added: BTreeSet<PeerId> = peers.difference(&network.peers).cloned().collect();
							let removed: BTreeSet<PeerId> = network.peers.difference(&peers).cloned().collect();

							// apply
							network.peers = peers.clone();

							// update co use handles
							for co in network.references.clone() {
								actions.push(ConnectionAction::PeersChanged(PeersChangedAction {
									id: co.clone(),
									peers: state.co_peers(&co),
									added: added.clone(),
									removed: removed.clone(),
								}));
							}
						},
						Err(err) => {
							tracing::warn!(?err, "connections-failed");
							// TODO: handle
						},
					}
				}
			},
			ConnectionAction::Disconnected(DisconnectedAction { network, reason: _ }) => {
				if let Some(mut network_connection) = state.networks.remove(network) {
					// remove references
					while let Some(co) = network_connection.references.pop_first() {
						if let Some(co_connection) = state.co.get_mut(&co) {
							if co_connection.networks.remove(network) {
								// update co use handles
								if co_connection.networks.is_empty() {
									// TODO: reconnect when not timedout yet?
									actions.push(ConnectionAction::Release(super::ReleaseAction { id: co.clone() }));
								} else {
									actions.push(ConnectionAction::PeersChanged(PeersChangedAction {
										id: co.clone(),
										removed: network_connection.peers.clone(),
										peers: state.co_peers(&co),
										added: [].into(),
									}));
								}
							}
						}
					}

					// remove disconnected
					state.co.retain(|_, co_connection| !co_connection.networks.is_empty());
				}
			},
			_ => {},
		}

		// result
		actions
	}
}
impl ConnectionState {
	/// Find all PeerId's for an CO.
	fn co_peers(&self, id: &CoId) -> BTreeSet<PeerId> {
		self.co
			.get(id)
			.iter()
			.flat_map(|co_connection| &co_connection.networks)
			.filter_map(|network| self.networks.get(network))
			.flat_map(|network_connection| network_connection.peers.clone())
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::ConnectionState;
	use crate::{
		actor::Reducer,
		plugins::connections::{ConnectAction, ConnectionAction, UseAction},
	};
	use co_primitives::{Network, NetworkPeer, NetworkRendezvous};
	use libp2p::PeerId;
	use std::{collections::BTreeSet, time::Instant, vec};

	#[test]
	fn test_use() {
		let mut state = ConnectionState::default();

		// setup
		let network1 = Network::Peer(NetworkPeer { peer: PeerId::random().to_bytes().to_vec(), addresses: vec![] });
		let network2 = Network::Rendezvous(NetworkRendezvous { namespace: "test".to_string(), addresses: vec![] });

		// connect
		let result = state.reduce(
			UseAction {
				from: "did:local:test".to_string(),
				id: "test".into(),
				time: Instant::now(),
				networks: [network1.clone(), network2.clone()].into_iter().collect(),
			}
			.into(),
		);
		assert_eq!(
			BTreeSet::from_iter(result),
			BTreeSet::from_iter([
				ConnectionAction::Connect(ConnectAction {
					network: network1.clone(),
					from: "did:local:test".to_owned()
				}),
				ConnectionAction::Connect(ConnectAction {
					network: network2.clone(),
					from: "did:local:test".to_owned()
				}),
			])
		);
		assert_eq!(state.co.len(), 1);
		assert_eq!(state.networks.len(), 2);

		// connect
		let result = state.reduce(
			UseAction {
				from: "did:local:test".to_string(),
				id: "test1".into(),
				time: Instant::now(),
				networks: [network2.clone()].into_iter().collect(),
			}
			.into(),
		);
		assert_eq!(result, vec![]); // already connecting
	}
}
