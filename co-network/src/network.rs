use crate::{
	bitswap::{BitswapMessage, BitswapStoreClient},
	didcomm, discovery,
	library::find_peer_id::try_peer_id,
	types::{network_task::TokioNetworkTaskSpawner, provider::BitswapBehaviourProvider},
	DidcommBehaviourProvider, DiscoveryLayerBehaviourProvider, FnOnceNetworkTask, GossipsubBehaviourProvider, Layer,
	LayerBehaviour, MdnsBehaviourProvider, NetworkError, NetworkTaskBox, NetworkTaskSpawner,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::ActorHandle;
use co_identity::{IdentityResolver, IdentityResolverBox, PrivateIdentityResolver, PrivateIdentityResolverBox};
use co_primitives::{DefaultParams, MultiCodec};
use futures::{pin_mut, Stream, StreamExt};
use libp2p::{
	dcutr, gossipsub, identify,
	identity::Keypair,
	mdns::{self, tokio::Behaviour as MdnsBehaviour},
	ping, relay,
	swarm::{behaviour::toggle::Toggle, dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use std::{collections::BTreeSet, task::Poll, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Span};

pub struct Libp2pNetwork {
	config: Libp2pNetworkConfig,
	shutdown: CancellationToken,
	tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<Behaviour, Context>>,
}
impl Libp2pNetwork {
	pub fn new<R, P>(
		identifier: String,
		config: Libp2pNetworkConfig,
		resolver: R,
		private_resolver: P,
		bitswap: ActorHandle<BitswapMessage<DefaultParams>>,
	) -> anyhow::Result<Libp2pNetwork>
	where
		R: IdentityResolver + Clone + Send + Sync + 'static,
		P: PrivateIdentityResolver + Clone + Send + Sync + 'static,
	{
		let resolver = IdentityResolverBox::new(resolver);
		let private_resolver = PrivateIdentityResolverBox::new(private_resolver);
		let local_peer_id = PeerId::from(config.keypair.public().clone());

		// kad
		// let kademlia_config: KademliaConfig = Default::default();
		// let kad = kad: Kademlia::with_config(local_peer_id.clone(), MemoryStore::new(local_peer_id.clone()),
		// kademlia_config);

		// gossipsub
		let gossipsub_config = gossipsub::ConfigBuilder::default()
			.max_transmit_size(256 * 1024)
			.build()
			.expect("valid config");
		let gossipsub =
			gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(config.keypair.clone()), gossipsub_config)
				.map_err(|err| anyhow!("gossip failed: {}", err))?;

		// bitswap
		let bitswap = Bitswap::<libipld::DefaultParams>::new(
			Default::default(),
			BitswapStoreClient::<DefaultParams>::new(bitswap),
			{
				let bitswap_identifier = identifier.clone();
				Box::new(move |t| {
					tokio::spawn(async move {
						t.instrument(tracing::trace_span!("bitswap", application = bitswap_identifier))
							.await
					});
				})
			},
		);

		// relay
		let relay = match config.mode {
			NetworkMode::Full => Some(libp2p::relay::Behaviour::new(local_peer_id.clone(), relay::Config::default())),
			_ => None,
		};

		// behaviour
		let behaviour = Behaviour {
			identify: identify::Behaviour::new(identify::Config::new("/ipfs/0.1.0".into(), config.keypair.public())),
			ping: ping::Behaviour::new(ping::Config::new()),
			mdns: MdnsBehaviour::new(mdns::Config::default(), local_peer_id.clone())?,
			// kad,
			bitswap,
			gossipsub,
			didcomm: didcomm::Behaviour::new(resolver.clone(), private_resolver, didcomm::Config { auto_dail: false }),
			dcutr: dcutr::Behaviour::new(local_peer_id.clone()),
			relay: relay.into(),
		};

		// // kad
		// for (peer, address) in config.bootstap.iter() {
		// 	behaviour.kad.add_address(peer, address.clone());
		// }
		// set_network_mode(&mut behaviour, config.mode);
		// if let Err(err) = behaviour.kad.bootstrap() {
		// 	tracing::warn!(?err, "kad-bootstrap-failed");
		// }

		let listen = config.addr.clone();
		let is_tcp = listen.protocol_stack().any(|protocol| protocol == "tcp");

		// swarm
		let swarm_builder = SwarmBuilder::with_existing_identity(config.keypair.clone()).with_tokio();
		let mut swarm = if is_tcp {
			swarm_builder
				.with_tcp(libp2p::tcp::Config::default(), libp2p::noise::Config::new, libp2p::yamux::Config::default)?
				.with_dns()?
				.with_behaviour(|_| behaviour)?
				.with_swarm_config(|config| config.with_idle_connection_timeout(Duration::from_secs(30)))
				.build()
		} else {
			swarm_builder
				.with_quic()
				.with_dns()?
				.with_behaviour(|_| behaviour)?
				.with_swarm_config(|config| config.with_idle_connection_timeout(Duration::from_secs(30)))
				.build()
		};

		// bootstrap
		for bootstrap in config.bootstrap.iter() {
			let peer_id = try_peer_id(bootstrap)?;
			if local_peer_id == peer_id {
				continue;
			}
			swarm.dial(DialOpts::peer_id(peer_id).addresses(vec![bootstrap.clone()]).build())?;
			swarm.behaviour_mut().gossipsub_mut().add_explicit_peer(&peer_id);
		}

		// context
		let context = Context {
			discovery: discovery::DiscoveryState::new(resolver.clone(), local_peer_id, Duration::from_secs(30), None),
		};

		// tasks
		let (tasks_tx, tasks_rx) = tokio::sync::mpsc::unbounded_channel();

		// runtime
		let shutdown = CancellationToken::new();
		let mut runtime = Runtime::new(config.clone(), shutdown.child_token());

		// listen
		runtime.listen(swarm.listen_on(listen)?);

		// run
		tokio::spawn(async move {
			run(swarm, context, runtime, tokio_stream::wrappers::UnboundedReceiverStream::new(tasks_rx))
				.instrument(tracing::trace_span!("network", application = identifier))
				.await;
		});

		// result
		Ok(Self { config, shutdown, tasks: tasks_tx })
	}

	pub fn spawner(&self) -> TokioNetworkTaskSpawner<Behaviour, Context> {
		TokioNetworkTaskSpawner { tasks: self.tasks.clone() }
	}

	/// Token to gracefully shutdown the network stack.
	/// This will stop accepting new connections and waits until established connections are done.
	pub fn shutdown(&self) -> Shutdown {
		Shutdown { shutdown: self.shutdown.clone() }
	}

	pub fn config(&self) -> &Libp2pNetworkConfig {
		&self.config
	}

	/// Change network mode.
	pub fn set_network_mode(&mut self, mode: NetworkMode) -> Result<(), NetworkError> {
		if self.config.mode != mode {
			self.config.mode = mode;
			self.spawner()
				.spawn(FnOnceNetworkTask::new(move |swarm, _| {
					set_network_mode(swarm.behaviour_mut(), mode);
				}))
				.unwrap();
		}
		Ok(())
	}
}
impl Drop for Libp2pNetwork {
	fn drop(&mut self) {
		self.shutdown.cancel();
	}
}

#[derive(Clone)]
pub struct Shutdown {
	shutdown: CancellationToken,
}
impl Shutdown {
	pub fn shutdown(&self) {
		tracing::info!("network-shutingdown");
		self.shutdown.cancel()
	}
}

#[derive(Clone, Debug)]
pub struct Libp2pNetworkConfig {
	pub keypair: Keypair,
	pub addr: Multiaddr,

	// Nodes to initialiy dial.
	/// The multiaddress is required to include an address (protocol) and and peer id (p2p).
	pub bootstrap: BTreeSet<Multiaddr>,

	/// Network mode to optimize for.
	/// This may change dynamically.
	/// For example when a mobile device gets plugged in to an power outlet.
	pub mode: NetworkMode,
}
impl Libp2pNetworkConfig {
	pub fn from_keypair(listen: Multiaddr, keypair: Keypair) -> Self {
		Self { keypair, addr: listen, bootstrap: Default::default(), mode: Default::default() }
	}
}

#[derive(Clone, Debug, Default, Copy, PartialEq)]
pub enum NetworkMode {
	#[default]
	Full,
	Light,
}

struct Runtime {
	_config: Libp2pNetworkConfig,
	listener_id: Option<libp2p::core::transport::ListenerId>,
	/// Tasks which have been executed but waiting for events.
	pending_tasks: Vec<(NetworkTaskBox<Behaviour, Context>, Span)>,
	shutdown: CancellationToken,
}
impl Runtime {
	fn new(config: Libp2pNetworkConfig, shutdown: CancellationToken) -> Self {
		Self { _config: config, listener_id: None, shutdown, pending_tasks: Default::default() }
	}

	fn listen(&mut self, id: libp2p::core::transport::ListenerId) {
		self.listener_id = Some(id);
	}

	fn is_running(&self) -> bool {
		!self.shutdown.is_cancelled()
	}
}

#[derive(Debug, Clone)]
pub enum ContextEvent {
	Discovery(discovery::Event),
}

#[derive(Debug, Clone)]
pub enum ContextLayerEvent {
	Discovery(discovery::DiscoveryEvent),
}

pub struct Context {
	pub discovery: discovery::DiscoveryState<IdentityResolverBox>,
}
impl LayerBehaviour<Behaviour> for Context {
	type ToSwarm = ContextEvent;
	type ToLayer = ContextLayerEvent;

	fn on_swarm_event(&mut self, event: &SwarmEvent<<Behaviour as NetworkBehaviour>::ToSwarm>) {
		LayerBehaviour::<Behaviour>::on_swarm_event(&mut self.discovery, event);
	}

	fn on_layer_event(&mut self, swarm: &mut Swarm<Behaviour>, event: Self::ToLayer) -> Option<Self::ToSwarm> {
		match event {
			ContextLayerEvent::Discovery(event) => self
				.discovery
				.on_layer_event(swarm, event)
				.map(|event| ContextEvent::Discovery(event)),
		}
	}

	fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Self::ToLayer> {
		match LayerBehaviour::<Behaviour>::poll(&mut self.discovery, cx) {
			Poll::Ready(event) => return Poll::Ready(ContextLayerEvent::Discovery(event)),
			Poll::Pending => {},
		}
		Poll::Pending
	}
}
impl DiscoveryLayerBehaviourProvider<IdentityResolverBox> for Context {
	type Event = NetworkEvent;

	fn discovery(&self) -> &discovery::DiscoveryState<IdentityResolverBox> {
		&self.discovery
	}

	fn discovery_mut(&mut self) -> &mut discovery::DiscoveryState<IdentityResolverBox> {
		&mut self.discovery
	}

	fn discovery_event(event: &Self::Event) -> Option<&discovery::Event> {
		match event {
			NetworkEvent::Discovery(event) => Some(event),
			_ => None,
		}
	}

	fn into_discovery_event(event: Self::Event) -> Result<discovery::Event, Self::Event> {
		match event {
			NetworkEvent::Discovery(event) => Ok(event),
			event => Err(event),
		}
	}
}

#[derive(Debug, derive_more::From)]
#[non_exhaustive]
pub enum NetworkEvent {
	Didcomm(didcomm::Event),
	Gossipsub(gossipsub::Event),
	Identify(identify::Event),
	Mdns(mdns::Event),
	Ping(ping::Event),
	// Kad(kad::Event),
	Bitswap(BitswapEvent),
	Discovery(discovery::Event),
	Dcutr(dcutr::Event),
	Relay(relay::Event),
}
// impl From<BehaviourEvent> for NetworkEvent {
// 	fn from(value: BehaviourEvent) -> Self {
// 		match value {
// 			BehaviourEvent::Didcomm(e) => NetworkEvent::Didcomm(e),
// 			BehaviourEvent::Gossipsub(e) => NetworkEvent::Gossipsub(e),
// 			BehaviourEvent::Identify(e) => NetworkEvent::Identify(e),
// 			BehaviourEvent::Mdns(e) => NetworkEvent::Mdns(e),
// 			BehaviourEvent::Ping(e) => NetworkEvent::Ping(e),
// 			BehaviourEvent::Kad(e) => NetworkEvent::Kad(e),
// 			BehaviourEvent::Bitswap(e) => NetworkEvent::Bitswap(e),
// 		}
// 	}
// }
impl From<ContextEvent> for NetworkEvent {
	fn from(value: ContextEvent) -> Self {
		match value {
			ContextEvent::Discovery(e) => NetworkEvent::Discovery(e),
		}
	}
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "NetworkEvent")]
pub struct Behaviour {
	pub didcomm: didcomm::Behaviour,
	pub gossipsub: gossipsub::Behaviour,
	pub identify: identify::Behaviour,
	pub mdns: MdnsBehaviour,
	pub ping: ping::Behaviour,
	// pub kad: Kademlia<MemoryStore>,
	pub bitswap: Bitswap<libipld::DefaultParams>,
	pub dcutr: dcutr::Behaviour,
	pub relay: Toggle<relay::Behaviour>,
}
impl discovery::DiscoveryBehaviour for Behaviour {
	fn rendezvous_client_mut(&mut self) -> Option<&mut libp2p::rendezvous::client::Behaviour> {
		None
	}

	fn rendezvous_client_event(
		_event: &<Self as NetworkBehaviour>::ToSwarm,
	) -> Option<&libp2p::rendezvous::client::Event> {
		None
	}

	fn mdns_mut(&mut self) -> Option<&mut mdns::tokio::Behaviour> {
		Some(&mut self.mdns)
	}

	fn mdns_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&mdns::Event> {
		match event {
			NetworkEvent::Mdns(e) => Some(e),
			_ => None,
		}
	}
}
impl DidcommBehaviourProvider for Behaviour {
	fn didcomm(&self) -> &didcomm::Behaviour {
		&self.didcomm
	}

	fn didcomm_mut(&mut self) -> &mut didcomm::Behaviour {
		&mut self.didcomm
	}

	fn didcomm_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&didcomm::Event> {
		match event {
			NetworkEvent::Didcomm(e) => Some(e),
			_ => None,
		}
	}

	fn into_didcomm_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<didcomm::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match event {
			NetworkEvent::Didcomm(e) => Ok(e),
			e => Err(e),
		}
	}
}
impl GossipsubBehaviourProvider for Behaviour {
	fn gossipsub(&self) -> &gossipsub::Behaviour {
		&self.gossipsub
	}

	fn gossipsub_mut(&mut self) -> &mut gossipsub::Behaviour {
		&mut self.gossipsub
	}

	fn gossipsub_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&gossipsub::Event> {
		match event {
			NetworkEvent::Gossipsub(e) => Some(e),
			_ => None,
		}
	}

	fn into_gossipsub_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<gossipsub::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match event {
			NetworkEvent::Gossipsub(e) => Ok(e),
			e => Err(e),
		}
	}
}
impl BitswapBehaviourProvider for Behaviour {
	fn bitswap(&self) -> &Bitswap<libipld::DefaultParams> {
		&self.bitswap
	}

	fn bitswap_mut(&mut self) -> &mut Bitswap<libipld::DefaultParams> {
		&mut self.bitswap
	}

	fn bitswap_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&BitswapEvent> {
		match event {
			NetworkEvent::Bitswap(e) => Some(e),
			_ => None,
		}
	}

	fn into_bitswap_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<BitswapEvent, <Self as NetworkBehaviour>::ToSwarm> {
		match event {
			NetworkEvent::Bitswap(e) => Ok(e),
			e => Err(e),
		}
	}
}
impl MdnsBehaviourProvider for Behaviour {
	fn mdns(&self) -> &mdns::tokio::Behaviour {
		&self.mdns
	}

	fn mdns_mut(&mut self) -> &mut mdns::tokio::Behaviour {
		&mut self.mdns
	}

	fn mdns_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&mdns::Event> {
		match event {
			NetworkEvent::Mdns(e) => Some(e),
			_ => None,
		}
	}

	fn into_mdns_event(
		event: <Self as NetworkBehaviour>::ToSwarm,
	) -> Result<mdns::Event, <Self as NetworkBehaviour>::ToSwarm> {
		match event {
			NetworkEvent::Mdns(e) => Ok(e),
			e => Err(e),
		}
	}
}

fn set_network_mode(_behaviour: &mut Behaviour, _mode: NetworkMode) {
	// match mode {
	// 	NetworkMode::Full => behaviour.kad.set_mode(Some(libp2p::kad::Mode::Server)),
	// 	NetworkMode::Light => behaviour.kad.set_mode(Some(libp2p::kad::Mode::Client)),
	// }
}

async fn run(
	mut swarm: Swarm<Behaviour>,
	context: Context,
	mut runtime: Runtime,
	tasks: impl Stream<Item = NetworkTaskBox<Behaviour, Context>>,
) {
	// log
	tracing::info!("network-running");

	// layer
	let mut context_layer = Layer::new(swarm.behaviour(), context);

	// handle
	let shutdown = runtime.shutdown.child_token();
	let mut shutdown_timeout = None;
	let tasks = tasks.fuse();
	pin_mut!(tasks);
	while runtime.is_running() {
		tokio::select! {
			// to not stack them up before creating new work
			// use biased as we always want to handle events first
			biased;

			// events
			_ = run_once(&mut swarm, &mut context_layer, &mut runtime) => {}

			// tasks
			Some(mut task) = tasks.next(), if !tasks.is_done() => {
				let task_span = tracing::trace_span!("network-task", ?task);

				// execute
				let task_complete = task_span.in_scope(|| {
					task.execute(&mut swarm, context_layer.layer_mut());
					task.is_complete()
				});

				// move to pending if not complete
				if !task_complete {
					// log
					task_span.in_scope(|| {
						tracing::trace!("network-task-pending");
					});

					// pending
					runtime.pending_tasks.push((task, task_span));
				}
			},

			// shutdown
			_ = shutdown.cancelled(), if shutdown_timeout.is_none() => {
				shutdown_timeout = Some(Duration::from_millis(1000));
			}
		}
	}

	// log
	tracing::info!("network-shutdown");
}

async fn run_once(swarm: &mut Swarm<Behaviour>, context: &mut Layer<Behaviour, Context>, runtime: &mut Runtime) {
	// event
	let network_event: Option<SwarmEvent<NetworkEvent>> = tokio::select! {
		swarm_event = swarm.select_next_some() => {
			context.on_swarm_event(&swarm_event);
			Some(swarm_event)
		},
		layer_event = context.select_next_some() => {
			context.on_layer_event(swarm, layer_event).map(|layer_event| SwarmEvent::Behaviour(NetworkEvent::from(layer_event)))
		},
	};

	// log
	// match &event {
	// 	SwarmEvent::NewListenAddr { address, .. } => {
	// 		tracing::info!(?address, "network-listening");
	// 	},
	// 	SwarmEvent::Behaviour(event) => {
	// 		tracing::debug!(?event, "network-behaviour-event");
	// 	},
	// 	event => {
	// 		tracing::debug!(?event, "network-event");
	// 	},
	// }

	// // known events
	// match &event {
	// 	SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns_event)) => handle_mdns(swarm, runtime, mdns_event),
	// 	_ => {},
	// }

	// tasks
	if let Some(event) = network_event {
		// log
		if is_log(&event) {
			tracing::trace!(?event, "network-event");
		}

		// tasks
		let mut result_event = Some(event);
		let mut task_index = 0;
		while task_index < runtime.pending_tasks.len() {
			// handle
			let task_complete = {
				let (task, task_span) = &mut runtime.pending_tasks[task_index];
				let _enter = task_span.enter();

				// run
				result_event = task.on_swarm_event(swarm, context.layer_mut(), result_event.unwrap());

				// complete?
				task.is_complete()
			};

			// done?
			if task_complete {
				// remove
				let (task, task_span) = runtime.pending_tasks.remove(task_index);
				task_index -= 1;

				// log
				task_span.in_scope(|| {
					tracing::trace!(?task, "network-task-completed");
				});
			}

			// event consumed?
			if result_event.is_none() {
				return;
			}

			// next
			task_index += 1;
		}

		// other
		if let Some(_event) = result_event {
			// ignore
		}
	}
}

fn is_log(event: &SwarmEvent<NetworkEvent>) -> bool {
	match event {
		SwarmEvent::Behaviour(NetworkEvent::Ping(_)) => false,
		_ => true,
	}
}
