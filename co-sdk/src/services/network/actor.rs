use super::message::NetworkMessage;
use crate::{
	local_keypair_fetch,
	services::{
		bitswap::Bitswap,
		connections::Connections,
		network::{CoNetworkTaskSpawner, MdnsGossipNetworkTask},
	},
	Action, CoContext,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, ActorInstance};
use co_network::{Libp2pNetwork, Libp2pNetworkConfig, NetworkTaskSpawner};
use co_primitives::{tags, Tags};
use libp2p::{Multiaddr, PeerId};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct NetworkSettings {
	pub force_new_peer_id: bool,
	pub listen: Option<Multiaddr>,
}
impl NetworkSettings {
	pub fn with_force_new_peer_id(mut self) -> Self {
		self.force_new_peer_id = true;
		self
	}

	pub fn with_listen_from_string(mut self, listen: &str) -> Result<Self, anyhow::Error> {
		self.listen = Some(listen.parse()?);
		Ok(self)
	}

	pub fn with_localhost(mut self) -> Self {
		self.listen = Some("/ip4/127.0.0.1/tcp/0".parse().unwrap());
		self
	}
}

#[derive(Debug)]
pub struct Network {
	context: CoContext,
}
impl Network {
	pub fn new(context: CoContext) -> Self {
		Self { context }
	}
}
#[async_trait]
impl Actor for Network {
	type Message = NetworkMessage;
	type State = NetworkState;
	type Initialize = NetworkSettings;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &Tags,
		settings: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		// bitswap
		let bitswap = Actor::spawn_with(
			self.context.tasks(),
			tags!("type": "bitswap", "application": self.context.identifier()),
			Bitswap::new(self.context.clone()),
			(),
		)?;

		// resolve key
		let local_identity = self.context.local_identity();
		let local_co = self.context.local_co_reducer().await?;
		let network_key =
			local_keypair_fetch(self.context.identifier(), &local_co, &local_identity, settings.force_new_peer_id)
				.await?;

		// network
		let network_peer_id = PeerId::from(network_key.public());
		let mut network_config = Libp2pNetworkConfig::from_keypair(network_key.clone());
		network_config.addr = settings.listen;
		let network = Libp2pNetwork::new(
			self.context.identifier().to_owned(),
			network_config,
			self.context.identity_resolver().await?,
			self.context.private_identity_resolver().await?,
			bitswap.handle(),
		)?;

		// spawner
		let spawner = CoNetworkTaskSpawner { spawner: network.spawner(), local_peer: network_peer_id.clone() };

		// connections
		let connections = Actor::spawn_with(
			self.context.tasks(),
			tags!("type": "connections", "application": self.context.identifier()),
			Connections::new(self.context.clone(), Duration::from_secs(30)),
			(),
		)?;

		// use mdns discoverd peers for gossip discovery
		spawner
			.spawn(MdnsGossipNetworkTask::new())
			.map_err(|err| ActorError::Actor(err.into()))?;

		// set network to reducers
		self.context
			.inner
			.set_network(Some((spawner.clone(), connections.handle())))
			.await?;

		// log
		tracing::info!(application = self.context.identifier(), peer_id = ?network_peer_id, "network");

		// reactive
		self.context.inner.application().dispatch(Action::NetworkStarted)?;

		// result
		Ok(NetworkState { network, peer_id: network_peer_id, connections, bitswap })
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
		Ok(())
	}
}

pub struct NetworkState {
	network: Libp2pNetwork,
	peer_id: PeerId,
	connections: ActorInstance<Connections>,
	bitswap: ActorInstance<Bitswap>,
}
