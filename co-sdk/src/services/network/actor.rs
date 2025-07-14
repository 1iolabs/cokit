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
use co_network::{try_peer_id, Libp2pNetwork, Libp2pNetworkConfig, NetworkTaskSpawner};
use co_primitives::{tags, Tags};
use libp2p::{Multiaddr, PeerId};
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetworkSettings {
	pub force_new_peer_id: bool,
	pub listen: Multiaddr,
	pub bootstrap: BTreeSet<Multiaddr>,
}
impl Default for NetworkSettings {
	fn default() -> Self {
		Self {
			force_new_peer_id: Default::default(),
			listen: Self::default_listen(),
			bootstrap: Self::default_bootstrap(),
		}
	}
}
impl NetworkSettings {
	pub fn new() -> Self {
		Self::default()
	}

	fn default_listen() -> Multiaddr {
		"/ip4/0.0.0.0/udp/0/quic-v1".parse().expect("to parse")
	}

	fn default_bootstrap() -> BTreeSet<Multiaddr> {
		let bootstrap =
			["/dns/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWCoAgVrvp9dWqk3bds1paFcrK8HuYB8yY13XWaahwfm7o"];
		bootstrap.into_iter().map(|s| s.parse().expect("to parse")).collect()
	}

	pub fn with_force_new_peer_id(mut self, value: bool) -> Self {
		self.force_new_peer_id = value;
		self
	}

	/// Set listen endpoint.
	pub fn with_listen(mut self, listen: Multiaddr) -> Self {
		self.listen = listen;
		self
	}

	/// Set listen endpoint.
	pub fn with_listen_from_string(mut self, listen: &str) -> Result<Self, anyhow::Error> {
		self.listen = listen.parse()?;
		Ok(self)
	}

	/// Set local listen endpoint.
	pub fn with_localhost(mut self) -> Self {
		self.listen = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
		self
	}

	/// Clear all bootstrap endpoints.
	pub fn without_bootstrap(mut self) -> Self {
		self.bootstrap.clear();
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstrap(mut self, bootstrap: Multiaddr) -> Self {
		self.bootstrap.insert(bootstrap);
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstraps(mut self, bootstrap: impl IntoIterator<Item = Multiaddr>) -> Self {
		self.bootstrap.extend(bootstrap);
		self
	}

	/// Add bootstrap endpoint.
	pub fn with_bootstrap_from_string(mut self, bootstrap: &str) -> Result<Self, anyhow::Error> {
		self.bootstrap.insert(bootstrap.parse()?);
		Ok(self)
	}

	/// Validate if settings are correct.
	pub fn build(self) -> Result<Self, anyhow::Error> {
		for bootstrap in self.bootstrap.iter() {
			try_peer_id(bootstrap)?;
		}
		Ok(self)
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
		let mut network_config = Libp2pNetworkConfig::from_keypair(settings.listen, network_key.clone());
		network_config.bootstrap = settings.bootstrap;
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
