use super::message::NetworkMessage;
use crate::{
	bitswap::BitswapMessage,
	services::{
		connections::Connections,
		heads::{HeadsActor, HeadsApi, HeadsContext},
		network::{CoNetworkTaskSpawner, ConnectionsNetworkTask, MdnsGossipNetworkTask, NetworkSettings},
	},
	try_peer_id, Libp2pNetwork, Libp2pNetworkConfig, NetworkTaskSpawner,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ActorInstance, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use co_primitives::{tags, DefaultParams, Tags};
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use std::{collections::BTreeSet, time::Duration};

pub struct NetworkInitialize {
	pub settings: NetworkSettings,
	pub identifier: String,
	pub keypair: Keypair,
	pub identity_resolver: IdentityResolverBox,
	pub private_identity_resolver: PrivateIdentityResolverBox,
	pub bitswap: ActorHandle<BitswapMessage<DefaultParams>>,
	pub tasks: TaskSpawner,
}

#[derive(Debug, Default)]
pub struct Network;
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
		// network
		let network_peer_id = PeerId::from(initialize.keypair.public());
		let mut network_config =
			Libp2pNetworkConfig::from_keypair(initialize.settings.listen, initialize.keypair.clone());
		network_config.bootstrap = initialize.settings.bootstrap;
		let network = Libp2pNetwork::new(
			initialize.identifier.clone(),
			network_config,
			initialize.identity_resolver,
			initialize.private_identity_resolver,
			initialize.bitswap,
		)?;

		// spawner
		let spawner = CoNetworkTaskSpawner { spawner: network.spawner(), local_peer: network_peer_id.clone() };

		// connections
		let connections = Actor::spawn_with(
			initialize.tasks.clone(),
			tags!("type": "connections", "application": &initialize.identifier),
			Connections::new(initialize.context.clone(), Duration::from_secs(30)),
			(),
		)?;
		spawner
			.spawn(ConnectionsNetworkTask::new(connections.handle()))
			.map_err(|err| ActorError::Actor(err.into()))?;

		// use mdns discoverd peers for gossip discovery
		spawner
			.spawn(MdnsGossipNetworkTask::new())
			.map_err(|err| ActorError::Actor(err.into()))?;

		// heads
		let heads = Actor::spawn_with(
			initialize.tasks.clone(),
			tags!("type": "heads", "application": &initialize.identifier),
			HeadsActor::default(),
			HeadsContext { network: spawner.clone(), spawner: initialize.tasks.clone() },
		)?;

		// log
		tracing::info!(application = initialize.identifier, peer_id = ?network_peer_id, "network");

		// result
		Ok(NetworkState { network, peer_id: network_peer_id, connections, heads })
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		// handle
		match message {
			NetworkMessage::Task(task) => {
				state.network.spawner().spawn_box(task).ok();
			},
			NetworkMessage::LocalPeerId(response) => {
				response.send(state.peer_id).ok();
			},
		}

		// result
		Ok(())
	}

	async fn shutdown(&self, state: Self::State) -> Result<(), ActorError> {
		state.network.shutdown().shutdown();
		state.connections.shutdown();
		state.bitswap.shutdown();
		state.heads.shutdown();
		Ok(())
	}
}

pub struct NetworkState {
	network: Libp2pNetwork,
	peer_id: PeerId,
	connections: ActorInstance<Connections>,
	bitswap: ActorInstance<Bitswap>,
	heads: ActorInstance<HeadsActor>,
}
