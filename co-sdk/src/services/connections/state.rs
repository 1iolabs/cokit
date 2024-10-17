use super::{
	action::{ConnectAction, ConnectedAction, ConnectionAction, DisconnectedAction, UseAction},
	DisconnectAction, NetworkResolveAction, NetworkResolvedAction, PeersChangedAction, ReleaseAction, ReleasedAction,
};
use crate::{actor::Reducer, services::connections::DisconnectReason};
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
	/// TODO: implement cache
	pub cache: HashMap<Network, BTreeSet<Multiaddr>>,
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
impl Reducer<ConnectionAction> for ConnectionState {
	fn reduce(&mut self, action: ConnectionAction) -> Vec<ConnectionAction> {
		let state = self;
		let mut actions = vec![];

		// state
		match &action {
			ConnectionAction::Use(UseAction { id, from, time, networks }) => {
				reduce_use(state, &mut actions, networks, id, from, time, true);
			},
			ConnectionAction::Connected(ConnectedAction { network, result }) => {
				reduce_connected(state, &mut actions, network, result);
			},
			ConnectionAction::NetworkResolved(NetworkResolvedAction { id, result, time }) => {
				reduce_network_resolved(state, &mut actions, id, result, time);
			},
			ConnectionAction::Disconnected(DisconnectedAction { network, reason: _ }) => {
				reduce_disconnected(state, &mut actions, network);
			},
			ConnectionAction::Release(ReleaseAction { id }) => {
				reduce_release(state, &mut actions, id);
			},
			ConnectionAction::Released(ReleasedAction { id }) => {
				reduce_released(state, &mut actions, id);
			},
			_ => {},
		}

		// result
		actions
	}
}

/// Use CO.
///
/// ## Responsibilities
/// - Create CO
/// - Resolve networks if not specified
/// - Connect networks that are not connected yet
/// - Notify about initial connections
///
/// TODO: use did? create new connections when different to existing?
/// TODO: The initial PeersChanged will be seen by all clients?
fn reduce_use(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	networks: &BTreeSet<Network>,
	id: &CoId,
	from: &String,
	time: &Instant,
	create: bool,
) {
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
			if create {
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
			} else {
				networks.clear();
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
				actions.push(ConnectionAction::Connect(ConnectAction { network: network.clone(), from: from.clone() }));
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
}

fn reduce_connected(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	network: &Network,
	result: &Result<BTreeSet<PeerId>, String>,
) {
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
				// log
				tracing::warn!(?err, network = ?network.network, peers_count = network.peers.len(), "connections-failed");

				// disconnected
				// TODO: retry connection?
				if network.peers.is_empty() {
					actions.push(ConnectionAction::Disconnected(DisconnectedAction {
						network: network.network.clone(),
						reason: DisconnectReason::Failure(err.to_string()),
					}));
				}
			},
		}
	}
}

fn reduce_network_resolved(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	id: &CoId,
	result: &Result<BTreeSet<Network>, String>,
	time: &Instant,
) {
	let networks = if let Some(co_connection) = state.co.get(id) {
		match result {
			Ok(networks) => Some((networks, co_connection.from.clone())),
			Err(_err) => {
				// when network resolve has been failed just release the co and let subscribers know it didn't work
				actions.push(ConnectionAction::Release(ReleaseAction { id: id.clone() }));
				None
			},
		}
	} else {
		None
	};
	if let Some((networks, from)) = networks {
		// populate networks
		reduce_use(state, actions, networks, id, &from, time, false);
	}
}

/// Network has been disconnected.
///
/// ## Responsibilities
/// - Release its references.
/// - Release Co is no more networks.
/// - Notify about peer changes.
fn reduce_disconnected(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, network: &Network) {
	if let Some(mut network_connection) = state.networks.remove(network) {
		// remove references
		while let Some(co) = network_connection.references.pop_first() {
			if let Some(co_connection) = state.co.get_mut(&co) {
				if co_connection.networks.remove(network) {
					// update co use handles
					if co_connection.networks.is_empty() {
						// TODO: reconnect when not timedout yet?
						actions.push(ConnectionAction::Released(ReleasedAction { id: co.clone() }));
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
}

/// Release a CO connection.
///
/// ## Responsibilities
/// - Disconnect networks which are only references by this CO
/// - Notify about ReleasedAction if no more networks connected (done by disconnected?)
fn reduce_release(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, id: &CoId) {
	if let Some(co_connection) = state.co.get_mut(id) {
		// remove references and disconnect if unused
		while let Some(network) = co_connection.networks.pop_first() {
			if let Some(network_connection) = state.networks.get_mut(&network) {
				if network_connection.references.remove(id) {
					if network_connection.references.is_empty() {
						actions.push(ConnectionAction::Disconnect(DisconnectAction { network }));
					}
				}
			}
		}

		// released
		if co_connection.networks.is_empty() {
			actions.push(ConnectionAction::Released(ReleasedAction { id: id.clone() }));
		}
	}
}

fn reduce_released(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, id: &CoId) {
	// remove co
	if let Some(mut co_connection) = state.co.remove(id) {
		// remove references and disconnect if unused
		// normally this should be empty at this point
		while let Some(network) = co_connection.networks.pop_first() {
			if let Some(network_connection) = state.networks.get_mut(&network) {
				if network_connection.references.remove(id) {
					if network_connection.references.is_empty() {
						actions.push(ConnectionAction::Disconnect(DisconnectAction { network }));
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::ConnectionState;
	use crate::{
		actor::Reducer,
		services::connections::{ConnectAction, ConnectionAction, UseAction},
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
