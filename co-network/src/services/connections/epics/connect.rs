// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{
	discovery,
	library::network_discovery::network_discovery,
	services::{
		connections::{
			action::{ConnectAction, ConnectedAction, ConnectionAction, DisconnectReason, DisconnectedAction},
			actor::ConnectionsContext,
			ConnectionState,
		},
		network::{DiscoveryConnectNetworkTask, ListnersNetworkTask},
	},
};
use co_actor::{Actions, Epic};
use co_identity::PrivateIdentityResolver;
use co_primitives::{Did, Network};
use futures::{Stream, StreamExt, TryStreamExt};
use std::collections::BTreeSet;

pub struct ConnectEpic();
impl ConnectEpic {
	pub fn new() -> Self {
		Self()
	}
}
impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> for ConnectEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
		message: &ConnectionAction,
		_state: &ConnectionState,
		context: &ConnectionsContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::Connect(ConnectAction { from, network }) => {
				Some(connect(context.clone(), from.clone(), network.clone()).map({
					let network = network.clone();
					move |item| match item {
						Ok(action) => Ok(action),
						Err(err) => Ok(ConnectionAction::Disconnected(DisconnectedAction {
							network: network.clone(),
							reason: DisconnectReason::Failure(err.to_string()),
						})),
					}
				}))
			},
			_ => None,
		}
	}
}

fn connect(
	context: ConnectionsContext,
	from: Did,
	network: Network,
) -> impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static {
	async_stream::try_stream! {
		// endpoints
		let endpoints = ListnersNetworkTask::listeners(&context.network, true, true).await?;

		// discovery
		let from_identity = context.private_identity_resolver.resolve_private(&from).await?;
		let discovery = network_discovery(Some(&context.identity_resolver), context.network.local_peer_id(), &from_identity, [network.clone()], [], endpoints).try_collect().await?;

		// connect
		let events = DiscoveryConnectNetworkTask::discover(context.network.clone(), discovery);

		// yield
		let mut peers = BTreeSet::new();
		for await event_result in events {
			let event = match event_result {
				Ok(event) => event,
				Err(err) => {
					yield ConnectionAction::Connected(ConnectedAction { network: network.clone(), result: Err(err.to_string()) });
					break;
				},
			};
			match event {
				discovery::Event::Connected { id: _, peer } => {
					if peers.insert(peer) {
						yield ConnectionAction::Connected(ConnectedAction { network: network.clone(), result: Ok(peers.clone()) });
					}
				},
				discovery::Event::Disconnected { id: _, peer, } => {
					if peers.remove(&peer) {
						yield ConnectionAction::Connected(ConnectedAction { network: network.clone(), result: Ok(peers.clone()) });
					}
				},
				discovery::Event::InsufficentPeers { id: _,  } => {
					yield ConnectionAction::InsufficentPeers;
				},
				discovery::Event::Timeout { id: _,  } => {
					yield ConnectionAction::Disconnected(DisconnectedAction { network: network.clone(), reason: DisconnectReason::Timeout });
				},
			}
		}
	}
}
