// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::message::NetworkMessage;
use crate::{
	bitswap::BitswapMessage,
	network::{Libp2pNetwork, Libp2pNetworkContext, CO_AGENT},
	services::{
		connections::{Connections, ConnectionsContext, DynamicNetworkResolver},
		discovery::{DiscoveryActor, DiscoveryApi, DiscoveryContext},
		dns::{DnsActor, DnsApi, DnsInitialize},
		heads::{HeadsActor, HeadsApi, HeadsContext},
		network::{
			tasks::{identify_dial::IdentifyDialNetworkTask, listen::ListenTask, recover::RecoverTask},
			CoNetworkTaskSpawner, ConnectionsNetworkTask, DiscoveryNetworkTask, NetworkApi, NetworkSettings,
		},
	},
	types::network_task::NetworkTaskSpawner,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ActorInstance, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use co_primitives::{tags, DynamicCoDate, Tags};
use libp2p::{identity::Keypair, PeerId};
use multiaddr::Protocol;

pub struct NetworkInitialize {
	pub settings: NetworkSettings,
	pub identifier: String,
	pub keypair: Keypair,
	pub date: DynamicCoDate,
	pub identity_resolver: IdentityResolverBox,
	pub private_identity_resolver: PrivateIdentityResolverBox,
	pub bitswap: ActorHandle<BitswapMessage>,
	/// Maximum accepted bitswap block size in bytes (wire-level DoS cap).
	pub max_block_size: usize,
	pub tasks: TaskSpawner,
	pub network_resolver: DynamicNetworkResolver,
}

#[derive(Debug, Default)]
pub struct Network;

fn spawn_identify_dial_task(spawner: &CoNetworkTaskSpawner, enabled: bool) -> Result<(), ActorError> {
	if enabled {
		spawner
			.spawn(IdentifyDialNetworkTask::new(CO_AGENT.to_string()))
			.map_err(|err| ActorError::Actor(err.into()))?;
	}
	Ok(())
}

#[async_trait]
impl Actor for Network {
	type Message = NetworkMessage;
	type State = NetworkState;
	type Initialize = NetworkInitialize;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let network_peer_id = PeerId::from(initialize.keypair.public());
		let dial_redundancy = initialize.settings.dial_redundancy;

		// dns
		let dns_spawner = DnsActor::spawner(tags!("type": "dns", "application": &initialize.identifier), DnsActor)?;
		let dns_handle = dns_spawner.handle();
		let dns_initialize = DnsInitialize::new(&initialize.settings.dns, dns_handle);
		let dns_resolver = dns_initialize.resolver();

		// network
		let network = Libp2pNetwork::new(
			Libp2pNetworkContext {
				identifier: initialize.identifier.clone(),
				tasks: initialize.tasks.clone(),
				resolver: initialize.identity_resolver.clone(),
				private_resolver: initialize.private_identity_resolver.clone(),
				bitswap: initialize.bitswap,
				max_block_size: initialize.max_block_size,
			},
			initialize.keypair.clone(),
			initialize.settings.clone(),
			dns_resolver,
		)
		.await?;

		let dns = dns_spawner.spawn(initialize.tasks.clone(), dns_initialize);

		// spawner
		let spawner = CoNetworkTaskSpawner { spawner: network.spawner(), local_peer: network_peer_id };

		// dial identified peer addresses
		spawn_identify_dial_task(&spawner, dial_redundancy)?;

		// use mdns discoverd peers for gossip discovery
		#[cfg(feature = "native")]
		spawner
			.spawn(super::MdnsGossipNetworkTask::new())
			.map_err(|err| ActorError::Actor(err.into()))?;

		// discovery
		let discovery_context = DiscoveryContext {
			tasks: initialize.tasks.clone(),
			network: spawner.clone(),
			date: initialize.date.clone(),
			resolver: initialize.identity_resolver.clone(),
			local_peer_id: network_peer_id,
			dial_redundancy,
		};
		let discovery = Actor::spawn_with(
			initialize.tasks.clone(),
			tags!("type": "discovery", "application": &initialize.identifier),
			DiscoveryActor::new(discovery_context),
			(),
		)?;
		spawner
			.spawn(DiscoveryNetworkTask::new(discovery.handle(), dial_redundancy))
			.map_err(|err| ActorError::Actor(err.into()))?;

		// connections
		let connections_context = ConnectionsContext {
			date: initialize.date.clone(),
			tasks: initialize.tasks.clone(),
			identity_resolver: initialize.identity_resolver.clone(),
			private_identity_resolver: initialize.private_identity_resolver.clone(),
			settings: initialize.settings.clone(),
			network: spawner.clone(),
			network_resolver: initialize.network_resolver,
			discovery: DiscoveryApi::from(&discovery),
		};
		let connections = Actor::spawn_with(
			initialize.tasks.clone(),
			tags!("type": "connections", "application": &initialize.identifier),
			Connections::new(connections_context),
			(),
		)?;
		spawner
			.spawn(ConnectionsNetworkTask::new(connections.handle()))
			.map_err(|err| ActorError::Actor(err.into()))?;

		// heads
		let heads = Actor::spawn_with(
			initialize.tasks.clone(),
			tags!("type": "heads", "application": &initialize.identifier),
			HeadsActor::default(),
			HeadsContext { network: spawner.clone(), spawner: initialize.tasks.clone() },
		)?;

		// keep the main listeners alive (browsers connect via relay, not direct listen)
		#[cfg(not(target_arch = "wasm32"))]
		for listen in initialize.settings.listen.iter() {
			spawner
				.spawn(ListenTask::new(listen.clone()))
				.map_err(|err| ActorError::Actor(err.into()))?;
		}

		// keep a relay-circuit listener alive per bootstrap
		for bootstrap in initialize.settings.bootstrap.iter() {
			spawner
				.spawn(ListenTask::new(bootstrap.clone().with(Protocol::P2pCircuit)))
				.map_err(|err| ActorError::Actor(err.into()))?;
		}

		// log
		tracing::info!(application = initialize.identifier, peer_id = ?network_peer_id, "network");

		// result
		Ok(NetworkState { network, peer_id: network_peer_id, discovery, connections, heads, dns })
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		// handle
		match message {
			NetworkMessage::LocalPeerId(response) => {
				response.respond(state.peer_id);
			},
			NetworkMessage::Network(response) => {
				response.respond(NetworkApi {
					spawner: CoNetworkTaskSpawner { spawner: state.network.spawner(), local_peer: state.peer_id },
					connections: state.connections.handle(),
					discovery: DiscoveryApi::from(&state.discovery),
					heads: HeadsApi::from(&state.heads),
					dns: DnsApi::from(&state.dns),
					_handle: handle.clone(),
				});
			},
			NetworkMessage::Recover(response) => {
				if let Err(err) = DnsApi::from(&state.dns).refresh() {
					tracing::warn!(?err, "network-dns-refresh-dispatch-failed");
				}
				if let Err(err) = state.network.spawner().spawn(RecoverTask) {
					tracing::warn!(?err, "network-recover-spawn-failed");
				}
				response.respond(());
			},
		}

		// result
		Ok(())
	}

	async fn shutdown(&self, state: Self::State) -> Result<(), ActorError> {
		state.network.shutdown().shutdown();
		state.connections.shutdown();
		state.discovery.shutdown();
		state.heads.shutdown();
		state.dns.shutdown();
		Ok(())
	}
}

pub struct NetworkState {
	network: Libp2pNetwork,
	peer_id: PeerId,
	discovery: ActorInstance<DiscoveryActor>,
	connections: ActorInstance<Connections>,
	heads: ActorInstance<HeadsActor>,
	dns: ActorInstance<DnsActor>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn identify_dial_task_is_only_spawned_when_enabled() {
		let spawner = CoNetworkTaskSpawner::new_closed(PeerId::random());

		assert!(spawn_identify_dial_task(&spawner, false).is_ok());
		assert!(spawn_identify_dial_task(&spawner, true).is_err());
	}
}
