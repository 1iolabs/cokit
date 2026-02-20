use super::message::NetworkMessage;
use crate::{
	bitswap::BitswapMessage,
	network::{Libp2pNetwork, CO_AGENT},
	services::{
		connections::{Connections, ConnectionsContext, DynamicNetworkResolver},
		heads::{HeadsActor, HeadsApi, HeadsContext},
		network::{
			tasks::{identify_dial::IdentifyDialNetworkTask, relay_listen::RelayListenTask},
			CoNetworkTaskSpawner, ConnectionsNetworkTask, MdnsGossipNetworkTask, NetworkApi, NetworkSettings,
		},
	},
	types::network_task::NetworkTaskSpawner,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ActorInstance, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use co_primitives::{tags, DynamicCoDate, Tags};
use libp2p::{identity::Keypair, PeerId};

pub struct NetworkInitialize {
	pub settings: NetworkSettings,
	pub identifier: String,
	pub keypair: Keypair,
	pub date: DynamicCoDate,
	pub identity_resolver: IdentityResolverBox,
	pub private_identity_resolver: PrivateIdentityResolverBox,
	pub bitswap: ActorHandle<BitswapMessage>,
	pub tasks: TaskSpawner,
	pub network_resolver: DynamicNetworkResolver,
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
		let network_peer_id = PeerId::from(initialize.keypair.public());

		// network
		let network = Libp2pNetwork::new(
			initialize.identifier.clone(),
			initialize.keypair.clone(),
			initialize.settings.clone(),
			initialize.date.clone(),
			initialize.identity_resolver.clone(),
			initialize.private_identity_resolver.clone(),
			initialize.bitswap,
		)?;

		// spawner
		let spawner = CoNetworkTaskSpawner { spawner: network.spawner(), local_peer: network_peer_id };

		// dial identified peer addresses
		spawner
			.spawn(IdentifyDialNetworkTask::new(CO_AGENT.to_string()))
			.map_err(|err| ActorError::Actor(err.into()))?;

		// use mdns discoverd peers for gossip discovery
		spawner
			.spawn(MdnsGossipNetworkTask::new())
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

		// use bootstraps as relay
		for bootstrap in initialize.settings.bootstrap.iter() {
			spawner
				.spawn(RelayListenTask::new(bootstrap.clone()))
				.map_err(|err| ActorError::Actor(err.into()))?;
		}

		// log
		tracing::info!(application = initialize.identifier, peer_id = ?network_peer_id, "network");

		// result
		Ok(NetworkState { network, peer_id: network_peer_id, connections, heads })
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
					heads: HeadsApi::from(&state.heads),
					_handle: handle.clone(),
				});
			},
		}

		// result
		Ok(())
	}

	async fn shutdown(&self, state: Self::State) -> Result<(), ActorError> {
		state.network.shutdown().shutdown();
		state.connections.shutdown();
		state.heads.shutdown();
		Ok(())
	}
}

pub struct NetworkState {
	network: Libp2pNetwork,
	peer_id: PeerId,
	connections: ActorInstance<Connections>,
	heads: ActorInstance<HeadsActor>,
}
