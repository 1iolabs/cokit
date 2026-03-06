// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::action::{
	ConnectAction, ConnectedAction, ConnectionAction, DidPeersChangedAction, DidReleaseAction, DidReleasedAction,
	DidUseAction, DisconnectAction, DisconnectReason, DisconnectedAction, NetworkResolveAction, NetworkResolvedAction,
	PeerConnectionClosedAction, PeerConnectionEstablishedAction, PeerRelateCoAction, PeerRelateDidAction,
	PeersChangedAction, ReleaseAction, ReleasedAction, UseAction,
};
use crate::{
	compat::Instant,
	connections::{DialAction, DialCompletedAction},
};
use co_actor::Reducer;
use co_primitives::{CoId, Did, Network, NetworkDidDiscovery, NetworkPeer};
use libp2p::{Multiaddr, PeerId};
use std::{
	collections::{BTreeSet, HashMap},
	time::Duration,
};

#[derive(Debug, Clone)]
pub struct CoConnection {
	pub id: CoId,
	pub from: Did,
	pub networks: BTreeSet<Network>,
	// pub keep_alive: Instant,
}

#[derive(Debug, Clone)]
pub struct DidConnection {
	pub to: Did,
	pub from: Did,
	pub networks: BTreeSet<Network>,
}

#[derive(Debug, Clone)]
pub struct NetworkConnection {
	pub network: Network,
	pub references: BTreeSet<CoId>,
	pub did_references: BTreeSet<Did>,
	pub peers: BTreeSet<PeerId>,
	/// TODO: implement cache
	pub keep_alive: Instant,
}

#[derive(Debug, Default, Clone)]
pub struct PeerConnection {
	pub connected: bool,
	pub co: BTreeSet<CoId>,
	pub network: BTreeSet<Network>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BootstrapPeer {
	pub peer_id: PeerId,
	pub endpoints: BTreeSet<Multiaddr>,
	pub failed_at: Option<Instant>,
	pub failed: u32,
	pub connecting: bool,
}
impl BootstrapPeer {
	pub fn new(peer_id: PeerId, endpoints: BTreeSet<Multiaddr>) -> Self {
		Self { peer_id, endpoints, failed: 0, connecting: false, failed_at: None }
	}
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionState {
	pub keep_alive: Duration,

	/// Tracks currently used cos and relates them with connectivity networks.
	pub co: HashMap<CoId, CoConnection>,

	/// Tracks currently used DID connections and relates them with connectivity networks.
	pub did: HashMap<Did, DidConnection>,

	/// Tracks currently used connectivity networks and relates them with cos and peers.
	pub networks: HashMap<Network, NetworkConnection>,

	/// Tracks currently connected peers and relates them with cos and networks.
	pub peers: HashMap<PeerId, PeerConnection>,

	/// Configured bootstrap peers.
	pub bootstrap: HashMap<PeerId, BootstrapPeer>,
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

	/// Find all PeerId's for a DID connection.
	fn did_peers(&self, to: &Did) -> BTreeSet<PeerId> {
		self.did
			.get(to)
			.iter()
			.flat_map(|did_connection| &did_connection.networks)
			.filter_map(|network| self.networks.get(network))
			.flat_map(|network_connection| network_connection.peers.clone())
			.collect()
	}

	/// Get initial use_did action.
	pub fn did_use_initial(&self, to: &Did) -> Option<DidPeersChangedAction> {
		let initial_peers = self.did_peers(to);
		if !initial_peers.is_empty() {
			Some(DidPeersChangedAction {
				to: to.clone(),
				peers: initial_peers.clone(),
				added: initial_peers,
				removed: Default::default(),
			})
		} else {
			None
		}
	}

	/// Get initial use action.
	pub fn use_initial(&self, id: &CoId) -> Option<PeersChangedAction> {
		// initial
		let initial_peers = self.co_peers(id);
		if !initial_peers.is_empty() {
			Some(PeersChangedAction {
				id: id.clone(),
				peers: initial_peers.clone(),
				added: initial_peers,
				removed: Default::default(),
			})
		} else {
			None
		}
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
			ConnectionAction::PeerConnectionEstablished(PeerConnectionEstablishedAction { peer_id, time }) => {
				reduce_peer_connection_established(state, &mut actions, *peer_id, time);
			},
			ConnectionAction::PeerConnectionClosed(PeerConnectionClosedAction { peer_id, time }) => {
				reduce_peer_connection_closed(state, &mut actions, *peer_id, time);
			},
			ConnectionAction::PeerRelateDid(action) => {
				reduce_peer_relate_did(state, &mut actions, action);
			},
			ConnectionAction::PeerRelateCo(action) => {
				reduce_peer_relate_co(state, &mut actions, action);
			},
			ConnectionAction::DidUse(action) => {
				reduce_did_use(state, &mut actions, action);
			},
			ConnectionAction::DidRelease(action) => {
				reduce_did_release(state, &mut actions, action);
			},
			ConnectionAction::DidReleased(action) => {
				reduce_did_released(state, &mut actions, action);
			},
			ConnectionAction::Dial(action) => {
				reduce_dial(state, &mut actions, action);
			},
			ConnectionAction::DialCompleted(action) => {
				reduce_dial_completed(state, &mut actions, action);
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
///
/// TODO: use did? create new connections when different to existing?
/// TODO: The initial PeersChanged will be seen by all clients?
fn reduce_use(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	networks: &BTreeSet<Network>,
	id: &CoId,
	from: &str,
	time: &Instant,
	create: bool,
) {
	let mut networks = networks.clone();

	// if no networks was specified we need to resolve them
	let networks_resolve = networks.is_empty();

	// use already connected peers
	let mut connected_networks = HashMap::new();
	for (peer_id, peer_connection) in state.peers.iter() {
		if peer_connection.connected {
			// by network
			for network in peer_connection.network.intersection(&networks) {
				// remember peer as connected
				connected_networks
					.entry(network.clone())
					.or_insert_with(BTreeSet::<PeerId>::new)
					.insert(*peer_id);
			}

			// by co
			//  here we create a "virtual" `NetworkPeer` as we don't know how the connection was made
			if peer_connection.co.contains(id) {
				// add as direct peer network
				let peer_network =
					Network::Peer(NetworkPeer { peer: peer_id.to_bytes(), addresses: Default::default() });
				networks.insert(peer_network.clone());

				// mark network as connected
				connected_networks
					.entry(peer_network)
					.or_insert_with(BTreeSet::<PeerId>::new)
					.insert(*peer_id);
			}
		}
	}

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
						from: from.to_owned(),
						// keep_alive: *time + state.keep_alive,
						networks: networks.clone(),
					},
				);

				// resolve networks if not specified
				if networks_resolve {
					actions.push(ConnectionAction::NetworkResolve(NetworkResolveAction { id: id.clone() }));
				}
			} else {
				networks.clear();
			}
		},
	}

	// network connections
	for network in networks.iter() {
		// already connected?
		let (connect, connected_peers) = if let Some(connected_network_peers) = connected_networks.remove(network) {
			(None, connected_network_peers)
		} else {
			(Some(from.to_owned()), Default::default())
		};

		// networks: get/create
		reference_network_connection(state, actions, network, id, connected_peers, connect, time);
	}
}

fn reduce_connected(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	network: &Network,
	result: &Result<BTreeSet<PeerId>, String>,
) {
	// get previous peer maps to create diffs
	let (network_co_peers, network_did_peers) = match state.networks.get(network) {
		Some(nc) => (
			Some(
				nc.references
					.iter()
					.map(|co| (co.clone(), state.co_peers(co)))
					.collect::<HashMap<CoId, BTreeSet<PeerId>>>(),
			),
			Some(
				nc.did_references
					.iter()
					.map(|did| (did.clone(), state.did_peers(did)))
					.collect::<HashMap<Did, BTreeSet<PeerId>>>(),
			),
		),
		None => (None, None),
	};

	// apply
	if let Some(network_connection) = state.networks.get_mut(network) {
		match result {
			Ok(peers) => {
				// extend
				network_connection.peers.extend(peers.iter().cloned());

				// relate
				peer_relate(state, network, None, peers.iter().cloned());
			},
			Err(err) => {
				// log
				tracing::warn!(?err, network = ?network_connection.network, peers_count = network_connection.peers.len(), "connections-failed");

				// disconnected
				// TODO: retry connection?
				if network_connection.peers.is_empty() {
					actions.push(ConnectionAction::Disconnected(DisconnectedAction {
						network: network_connection.network.clone(),
						reason: DisconnectReason::Failure(err.to_string()),
					}));
				}
			},
		}
	}

	// update co handles
	if let Some(network_co_peers) = network_co_peers {
		for (co, previous_co_peers) in network_co_peers {
			let next_co_peers = state.co_peers(&co);
			let added: BTreeSet<PeerId> = next_co_peers.difference(&previous_co_peers).cloned().collect();
			let removed: BTreeSet<PeerId> = previous_co_peers.difference(&next_co_peers).cloned().collect();
			if !removed.is_empty() || !added.is_empty() {
				actions.push(ConnectionAction::PeersChanged(PeersChangedAction {
					id: co.clone(),
					peers: next_co_peers,
					added,
					removed,
				}));
			}
		}
	}

	// update did handles
	if let Some(network_did_peers) = network_did_peers {
		for (did, previous_did_peers) in network_did_peers {
			let next_did_peers = state.did_peers(&did);
			let added: BTreeSet<PeerId> = next_did_peers.difference(&previous_did_peers).cloned().collect();
			let removed: BTreeSet<PeerId> = previous_did_peers.difference(&next_did_peers).cloned().collect();
			if !removed.is_empty() || !added.is_empty() {
				actions.push(ConnectionAction::DidPeersChanged(DidPeersChangedAction {
					to: did.clone(),
					peers: next_did_peers,
					added,
					removed,
				}));
			}
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
			Ok(new_networks) => Some((new_networks, co_connection.from.clone())),
			Err(_err) => {
				// when network resolve has been failed just release the co and let subscribers know it didn't work
				// if we had no networks before
				if co_connection.networks.is_empty() {
					actions.push(ConnectionAction::Release(ReleaseAction { id: co_connection.id.clone() }));
				}

				// nothing to add
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
					} else if !network_connection.peers.is_empty() {
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

		// remove did references
		while let Some(did) = network_connection.did_references.pop_first() {
			if let Some(did_connection) = state.did.get_mut(&did) {
				if did_connection.networks.remove(network) {
					if did_connection.networks.is_empty() {
						actions.push(ConnectionAction::DidReleased(DidReleasedAction { to: did.clone() }));
					} else if !network_connection.peers.is_empty() {
						actions.push(ConnectionAction::DidPeersChanged(DidPeersChangedAction {
							to: did.clone(),
							removed: network_connection.peers.clone(),
							peers: state.did_peers(&did),
							added: [].into(),
						}));
					}
				}
			}
		}

		// remove disconnected
		state.co.retain(|_, co_connection| !co_connection.networks.is_empty());
		state.did.retain(|_, did_connection| !did_connection.networks.is_empty());
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
				if network_connection.references.remove(id)
					&& network_connection.references.is_empty()
					&& network_connection.did_references.is_empty()
				{
					actions.push(ConnectionAction::Disconnect(DisconnectAction { network }));
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
				if network_connection.references.remove(id)
					&& network_connection.references.is_empty()
					&& network_connection.did_references.is_empty()
				{
					actions.push(ConnectionAction::Disconnect(DisconnectAction { network }));
				}
			}
		}
	}
}

fn reduce_peer_connection_established(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	peer_id: PeerId,
	time: &Instant,
) {
	// mark as connected
	let (cos, networks) = {
		let peer_connection = state.peers.entry(peer_id).or_default();
		peer_connection.connected = true;
		(peer_connection.co.clone(), peer_connection.network.clone())
	};

	// reference networks (if it is currently in use)
	for network in networks {
		if state.networks.contains_key(&network) {
			reduce_connected(state, actions, &network, &Ok([peer_id].into()));
		}
	}

	// reference direct network (if co is currently in use)
	let network = Network::Peer(NetworkPeer { peer: peer_id.to_bytes(), addresses: Default::default() });
	for co in cos {
		if state.co.contains_key(&co) {
			reference_network_connection(state, actions, &network, &co, [peer_id].into(), None, time);
		}
	}

	// let mut networks_connected = BTreeSet::new();

	// // connected
	// {
	// 	let peer_connection = state.peers.entry(peer_id).or_insert_with(PeerConnection::default);
	// 	let was_disconnected = peer_connection.connected == false;

	// 	// set as connected
	// 	peer_connection.connected = true;

	// 	// add network peers if this connection is newly established
	// 	if was_disconnected {
	// 		for network in peer_connection.network.iter() {
	// 			if let Some(network_connection) = state.networks.get(network) {
	// 				if !network_connection.peers.contains(&peer_id) {
	// 					networks_connected.insert(network.clone());
	// 				}
	// 			}
	// 		}
	// 	}
	// }

	// // add default peer network
	// let peer_network = Network::Peer(NetworkPeer { peer: peer_id.to_bytes(), addresses: Default::default() });
	// if !state.networks.contains_key(&peer_network) {
	// 	state.networks.insert(
	// 		peer_network.clone(),
	// 		NetworkConnection {
	// 			network: peer_network.clone(),
	// 			references: Default::default(),
	// 			peers: [peer_id].into(),
	// 			keep_alive: *time + state.keep_alive,
	// 		},
	// 	);
	// 	networks_connected.insert(peer_network);
	// }

	// // networks
	// for network_connected in networks_connected {
	// 	reduce_connected(state, actions, &network_connected, &Ok([peer_id].into()));
	// }
}

fn reduce_peer_connection_closed(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	peer_id: PeerId,
	_time: &Instant,
) {
	let mut peers_changed = BTreeSet::new();
	let mut networks_disconnected = BTreeSet::new();

	// disconnect
	if let Some(peer_connection) = state.peers.get_mut(&peer_id) {
		// set as disconnected
		peer_connection.connected = false;

		// disconnect from networks
		for network in peer_connection.network.iter() {
			if let Some(network_connection) = state.networks.get_mut(network) {
				if network_connection.peers.remove(&peer_id) {
					peers_changed.extend(network_connection.references.iter().cloned());
				}
				if network_connection.peers.is_empty() {
					networks_disconnected.insert(network.clone());
				}
			}
		}
	}

	// handle disconnected networks
	for network_disconnected in networks_disconnected {
		reduce_disconnected(state, actions, &network_disconnected);
	}

	// handle peer changes
	for co in peers_changed {
		actions.push(ConnectionAction::PeersChanged(PeersChangedAction {
			id: co.clone(),
			removed: [peer_id].into(),
			peers: state.co_peers(&co),
			added: [].into(),
		}));
	}
}

fn reduce_peer_relate_did(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	action: &PeerRelateDidAction,
) {
	// TODO: what if a different topic is used?
	let network = Network::DidDiscovery(NetworkDidDiscovery { topic: None, did: action.did.clone() });
	peer_relate(state, &network, None, [action.peer_id]);

	// reference
	let peer_connected = state
		.peers
		.get(&action.peer_id)
		.map(|peer_connection| peer_connection.connected)
		.unwrap_or(false);
	if peer_connected {
		let cos = if let Some(network_connection) = state.networks.get(&network) {
			network_connection.references.clone()
		} else {
			Default::default()
		};
		for co in cos {
			reference_network_connection(state, actions, &network, &co, [action.peer_id].into(), None, &action.time);
		}
	}
}

fn reduce_peer_relate_co(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	action: &PeerRelateCoAction,
) {
	// did
	if let Some(did) = &action.did {
		reduce_peer_relate_did(
			state,
			actions,
			&PeerRelateDidAction { peer_id: action.peer_id, did: did.to_owned(), time: action.time },
		);
	}

	// co
	let network = Network::Peer(NetworkPeer { peer: action.peer_id.to_bytes(), addresses: Default::default() });
	peer_relate(state, &network, Some(&action.co), [action.peer_id]);

	// reference
	let peer_connected = state
		.peers
		.get(&action.peer_id)
		.map(|peer_connection| peer_connection.connected)
		.unwrap_or(false);
	if peer_connected {
		reference_network_connection(state, actions, &network, &action.co, [action.peer_id].into(), None, &action.time);
	}
}

fn reduce_did_use(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, action: &DidUseAction) {
	let to = &action.to;

	// get or create did connection
	let new_networks = match state.did.get_mut(to) {
		Some(did_connection) => {
			let mut new = BTreeSet::new();
			for network in &action.networks {
				if did_connection.networks.insert(network.clone()) {
					new.insert(network.clone());
				}
			}
			new
		},
		None => {
			state.did.insert(
				to.clone(),
				DidConnection { from: action.from.clone(), to: to.clone(), networks: action.networks.clone() },
			);
			action.networks.clone()
		},
	};

	// connect new networks
	for network in &new_networks {
		// check if already connected with peers
		let (connect, connected_peers) = if let Some(nc) = state.networks.get(network) {
			if nc.peers.is_empty() {
				(None, Default::default())
			} else {
				(None, nc.peers.clone())
			}
		} else {
			(Some(action.from.clone()), Default::default())
		};

		// reference network for this DID
		reference_did_network_connection(state, actions, network, to, connected_peers, connect, &action.time);
	}
}

fn reduce_did_release(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, action: &DidReleaseAction) {
	let id = &action.to;
	if let Some(did_connection) = state.did.get_mut(id) {
		let mut networks = std::mem::take(&mut did_connection.networks);
		while let Some(network) = networks.pop_first() {
			if let Some(network_connection) = state.networks.get_mut(&network) {
				if network_connection.did_references.remove(id)
					&& network_connection.references.is_empty()
					&& network_connection.did_references.is_empty()
				{
					actions.push(ConnectionAction::Disconnect(DisconnectAction { network }));
				}
			}
		}
		actions.push(ConnectionAction::DidReleased(DidReleasedAction { to: id.clone() }));
	}
}

fn reduce_did_released(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, action: &DidReleasedAction) {
	// remove did
	let id = &action.to;
	if let Some(mut connection) = state.did.remove(id) {
		// remove references and disconnect if unused
		// normally this should be empty at this point
		while let Some(network) = connection.networks.pop_first() {
			if let Some(network_connection) = state.networks.get_mut(&network) {
				if network_connection.did_references.remove(id)
					&& network_connection.references.is_empty()
					&& network_connection.did_references.is_empty()
				{
					actions.push(ConnectionAction::Disconnect(DisconnectAction { network }));
				}
			}
		}
	}
}

/// Update/Create NetworkConnection and relate it with a DID.
fn reference_did_network_connection(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	network: &Network,
	did: &Did,
	connected_peers: BTreeSet<PeerId>,
	connect: Option<Did>,
	time: &Instant,
) {
	let did_has_network = match state.did.get(did) {
		Some(dc) => dc.networks.contains(network),
		None => return,
	};
	let (network_has_peers, network_has_did_ref) = match state.networks.get(network) {
		Some(nc) => (nc.peers.is_superset(&connected_peers), nc.did_references.contains(did)),
		None => (false, false),
	};

	let previous_did_peers =
		if !did_has_network || !network_has_did_ref || !network_has_peers { Some(state.did_peers(did)) } else { None };

	// add network to did
	if !did_has_network {
		if let Some(dc) = state.did.get_mut(did) {
			dc.networks.insert(network.clone());
		}
	}

	// add did ref to network or create
	if !network_has_did_ref || !network_has_peers {
		match state.networks.get_mut(network) {
			Some(nc) => {
				nc.did_references.insert(did.clone());
				nc.peers.extend(connected_peers.iter().cloned());
				nc.keep_alive = *time + state.keep_alive;
			},
			None => {
				let nc = NetworkConnection {
					keep_alive: *time + state.keep_alive,
					network: network.clone(),
					peers: connected_peers,
					references: Default::default(),
					did_references: [did.clone()].into(),
				};
				state.networks.insert(network.clone(), nc);

				if let Some(from) = connect {
					actions.push(ConnectionAction::Connect(ConnectAction { network: network.clone(), from }));
				}
			},
		}
	}

	// notify
	if let Some(previous_did_peers) = previous_did_peers {
		let next_did_peers = state.did_peers(did);
		let added: BTreeSet<PeerId> = next_did_peers.difference(&previous_did_peers).cloned().collect();
		let removed: BTreeSet<PeerId> = previous_did_peers.difference(&next_did_peers).cloned().collect();
		if !removed.is_empty() || !added.is_empty() {
			actions.push(ConnectionAction::DidPeersChanged(DidPeersChangedAction {
				to: did.clone(),
				peers: next_did_peers,
				added,
				removed,
			}));
		}
	}
}

fn reduce_dial(state: &mut ConnectionState, _actions: &mut Vec<ConnectionAction>, action: &DialAction) {
	if let Some(bootstrap) = state.bootstrap.get_mut(&action.peer_id) {
		bootstrap.connecting = true;
	}
}

fn reduce_dial_completed(
	state: &mut ConnectionState,
	_actions: &mut Vec<ConnectionAction>,
	action: &DialCompletedAction,
) {
	if let Some(bootstrap) = state.bootstrap.get_mut(&action.peer_id) {
		bootstrap.connecting = false;
		if action.ok {
			bootstrap.failed_at = None;
			bootstrap.failed = 0;
		} else {
			bootstrap.failed_at = Some(action.time);
			bootstrap.failed += 1;
		}
	}
}

// /// Validate all direct connections to a peer.
// fn peer_connect(state: &mut ConnectionState, actions: &mut Vec<ConnectionAction>, time: &Instant, peer_id: PeerId) {
// 	let mut networks_connected = BTreeSet::new();

// 	// find connected networks
// 	if let Some(peer_connection) = state.peers.get(&peer_id) {
// 		if peer_connection.connected {
// 			for co in &peer_connection.co {

// 			}

// 			// ensure that all networks that are related to this peer will be connected
// 			for network in peer_connection.network.iter() {
// 				if let Some(network_connection) = state.networks.get(network) {
// 					if !network_connection.peers.contains(&peer_id) {
// 						networks_connected.insert(network.clone());
// 					}
// 				}
// 			}

// 			// add default peer network
// 			let peer_network = Network::Peer(NetworkPeer { peer: peer_id.to_bytes(), addresses: Default::default() });
// 			if let Some(network_connection) = state.networks.get_mut(&peer_network) {
// 				for co in &peer_connection.co {
// 					if !network_connection.references.contains(co) {
// 						network_connection.references.insert(co.clone());
// 						networks_connected.insert(peer_network);
// 					}
// 				}
// 			} else {
// 				state.networks.insert(
// 					peer_network.clone(),
// 					NetworkConnection {
// 						network: peer_network.clone(),
// 						references: peer_connection.co.clone(),
// 						peers: [peer_id].into(),
// 						keep_alive: *time + state.keep_alive,
// 					},
// 				);
// 				networks_connected.insert(peer_network);
// 			}
// 		}
// 	}

// 	// networks
// 	for network_connected in networks_connected {
// 		reduce_connected(state, actions, &network_connected, &Ok([peer_id].into()));
// 	}
// }

/// Relate peers with a network and optionally a co.
/// This will only mutate `state.peers`.
fn peer_relate(
	state: &mut ConnectionState,
	network: &Network,
	co: Option<&CoId>,
	peers: impl IntoIterator<Item = PeerId>,
) {
	for peer in peers.into_iter() {
		if let Some(peer_connection) = state.peers.get_mut(&peer) {
			// co
			if let Some(co) = co {
				peer_connection.co.insert(co.clone());
			}
			if let Some(network_connection) = state.networks.get(network) {
				peer_connection.co.extend(network_connection.references.iter().cloned());
			}

			// network
			if !peer_connection.network.contains(network) {
				peer_connection.network.insert(network.clone());
			}
		}
	}
}

/// Update/Create NetworkConnection and relate it with an Co.
/// If the Co is not currently in use this does nothing.
fn reference_network_connection(
	state: &mut ConnectionState,
	actions: &mut Vec<ConnectionAction>,
	network: &Network,
	co: &CoId,
	connected_peers: BTreeSet<PeerId>,
	connect: Option<Did>,
	time: &Instant,
) {
	let co_has_network_reference = match state.co.get(co) {
		Some(co_connection) => co_connection.networks.contains(network),
		None => {
			// skip as co is not is use
			return;
		},
	};
	let (network_has_co_reference, network_has_peer_references) = match state.networks.get(network) {
		Some(network_connection) => {
			(network_connection.peers.is_superset(&connected_peers), network_connection.references.contains(co))
		},
		None => (false, false),
	};

	// get current peers
	let previous_co_peers = if !co_has_network_reference || !network_has_co_reference || !network_has_peer_references {
		Some(state.co_peers(co))
	} else {
		None
	};

	// add network to co reference
	if !co_has_network_reference {
		if let Some(co_connection) = state.co.get_mut(co) {
			co_connection.networks.insert(network.clone());
		}
	}

	// add co to network references or create new network connection
	if !network_has_co_reference || !network_has_peer_references {
		match state.networks.get_mut(network) {
			Some(network_connection) => {
				// reference network
				network_connection.references.insert(co.clone());
				network_connection.peers.extend(connected_peers.iter().cloned());
				network_connection.keep_alive = *time + state.keep_alive;
			},
			None => {
				let network_connection = NetworkConnection {
					keep_alive: *time + state.keep_alive,
					network: network.clone(),
					peers: connected_peers,
					references: [co.clone()].into(),
					did_references: Default::default(),
				};

				// insert
				state.networks.insert(network.clone(), network_connection);

				// connect
				if let Some(from) = connect {
					actions.push(ConnectionAction::Connect(ConnectAction { network: network.clone(), from }));
				}
			},
		}
	}

	// notify
	if let Some(previous_co_peers) = previous_co_peers {
		let next_co_peers = state.co_peers(co);
		let added: BTreeSet<PeerId> = next_co_peers.difference(&previous_co_peers).cloned().collect();
		let removed: BTreeSet<PeerId> = previous_co_peers.difference(&next_co_peers).cloned().collect();
		if !removed.is_empty() || !added.is_empty() {
			actions.push(ConnectionAction::PeersChanged(PeersChangedAction {
				id: co.clone(),
				peers: next_co_peers,
				added,
				removed,
			}));
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::connections::{
		ConnectAction, ConnectedAction, ConnectionAction, ConnectionState, DidPeersChangedAction, DidReleaseAction,
		DidReleasedAction, DidUseAction, DisconnectAction, PeerConnectionEstablishedAction, PeerRelateDidAction,
		UseAction,
	};
	use co_actor::Reducer;
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

	#[test]
	fn test_peer_relate_did() {
		let mut state = ConnectionState::default();
		let peer_id = PeerId::random();
		let did = "did:local:test".to_string();

		// add peer as connected
		let action = ConnectionAction::PeerConnectionEstablished(PeerConnectionEstablishedAction {
			peer_id,
			time: Instant::now(),
		});
		state.reduce(action);
		assert!(state.peers.contains_key(&peer_id));

		// relate
		let action =
			ConnectionAction::PeerRelateDid(PeerRelateDidAction { peer_id, did: did.clone(), time: Instant::now() });
		state.reduce(action);
		assert!(state.peers[&peer_id]
			.network
			.iter()
			.any(|network| matches!(network, Network::DidDiscovery(did_discovery) if did_discovery.did == did)));
	}

	#[test]
	fn test_did_use_connects_networks() {
		let mut state = ConnectionState::default();
		let network1 = Network::Rendezvous(NetworkRendezvous { namespace: "n1".to_string(), addresses: vec![] });
		let network2 = Network::Rendezvous(NetworkRendezvous { namespace: "n2".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();

		let result = state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network1.clone(), network2.clone()].into_iter().collect(),
		}));

		// should emit Connect for both networks
		assert_eq!(
			BTreeSet::from_iter(result),
			BTreeSet::from_iter([
				ConnectionAction::Connect(ConnectAction { network: network1.clone(), from: from.clone() }),
				ConnectionAction::Connect(ConnectAction { network: network2.clone(), from: from.clone() }),
			])
		);
		assert_eq!(state.did.len(), 1);
		assert_eq!(state.did[&to].networks.len(), 2);
		assert_eq!(state.networks.len(), 2);
		assert!(state.networks[&network1].did_references.contains(&to));
		assert!(state.networks[&network2].did_references.contains(&to));
	}

	#[test]
	fn test_did_use_reuses_existing_network() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "n1".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();

		// first DidUse creates the network
		state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));

		// second DidUse for same DID+network should not re-connect
		let result = state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));
		assert_eq!(result, vec![]);
	}

	#[test]
	fn test_did_use_peers_changed_on_connected() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "n1".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();
		let peer = PeerId::random();

		// setup did connection
		state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));

		// simulate network connected with a peer
		let result = state.reduce(ConnectionAction::Connected(ConnectedAction {
			network: network.clone(),
			result: Ok([peer].into()),
		}));

		// should emit DidPeersChanged
		assert_eq!(
			result,
			vec![ConnectionAction::DidPeersChanged(DidPeersChangedAction {
				to: to.clone(),
				peers: [peer].into(),
				added: [peer].into(),
				removed: Default::default(),
			})]
		);
		assert_eq!(state.did_peers(&to), [peer].into());
	}

	#[test]
	fn test_did_use_shared_network_with_co() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "shared".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();

		// CO uses network first
		state.reduce(ConnectionAction::Use(UseAction {
			from: from.clone(),
			id: "co1".into(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));
		assert_eq!(state.networks.len(), 1);

		// DID uses same network — should not emit Connect (already exists)
		let result = state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));
		// no Connect action since network already exists
		assert!(result.iter().all(|a| !matches!(a, ConnectionAction::Connect(_))));
		let co1: co_primitives::CoId = "co1".into();
		assert!(state.networks[&network].references.contains(&co1));
		assert!(state.networks[&network].did_references.contains(&to));
	}

	#[test]
	fn test_did_release_disconnects_exclusive_network() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "n1".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();

		state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));

		let result = state.reduce(ConnectionAction::DidRelease(DidReleaseAction { to: to.clone() }));

		assert_eq!(
			BTreeSet::from_iter(result),
			BTreeSet::from_iter([
				ConnectionAction::Disconnect(DisconnectAction { network: network.clone() }),
				ConnectionAction::DidReleased(DidReleasedAction { to: to.clone() }),
			])
		);
	}

	#[test]
	fn test_did_release_preserves_shared_network() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "shared".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();

		// CO uses network
		state.reduce(ConnectionAction::Use(UseAction {
			from: from.clone(),
			id: "co1".into(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));

		// DID also uses same network
		state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));

		// releasing DID should NOT disconnect the shared network
		let result = state.reduce(ConnectionAction::DidRelease(DidReleaseAction { to: to.clone() }));
		assert_eq!(result, vec![ConnectionAction::DidReleased(DidReleasedAction { to: to.clone() })]);
		assert!(state.networks.contains_key(&network));
		assert!(!state.networks[&network].did_references.contains(&to));
	}

	#[test]
	fn test_did_released_cleans_up_state() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "n1".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();

		state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));
		assert!(state.did.contains_key(&to));

		// release + released
		state.reduce(ConnectionAction::DidRelease(DidReleaseAction { to: to.clone() }));
		state.reduce(ConnectionAction::DidReleased(DidReleasedAction { to: to.clone() }));

		assert!(!state.did.contains_key(&to));
	}

	#[test]
	fn test_did_use_initial_peers() {
		let mut state = ConnectionState::default();
		let network = Network::Rendezvous(NetworkRendezvous { namespace: "n1".to_string(), addresses: vec![] });
		let from = "did:local:alice".to_string();
		let to = "did:local:bob".to_string();
		let peer = PeerId::random();

		// setup: did_use + connected
		state.reduce(ConnectionAction::DidUse(DidUseAction {
			from: from.clone(),
			to: to.clone(),
			time: Instant::now(),
			networks: [network.clone()].into_iter().collect(),
		}));
		state.reduce(ConnectionAction::Connected(ConnectedAction {
			network: network.clone(),
			result: Ok([peer].into()),
		}));

		// did_use_initial should return current peers
		let initial = state.did_use_initial(&to);
		assert_eq!(
			initial,
			Some(DidPeersChangedAction {
				to: to.clone(),
				peers: [peer].into(),
				added: [peer].into(),
				removed: Default::default(),
			})
		);

		// unknown DID should return None
		assert_eq!(state.did_use_initial(&"did:local:unknown".to_string()), None);
	}
}
