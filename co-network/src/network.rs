use crate::{
	bitswap::{BitswapMessage, BitswapStoreClient},
	didcomm, discovery, heads,
	types::{network_task::TokioNetworkTaskSpawner, provider::BitswapBehaviourProvider},
	DidcommBehaviourProvider, DiscoveryLayerBehaviourProvider, FnOnceNetworkTask, GossipsubBehaviourProvider,
	HeadsLayerBehaviourProvider, Layer, LayerBehaviour, MdnsBehaviourProvider, NetworkError, NetworkTaskBox,
	NetworkTaskSpawner,
};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_identity::{IdentityResolver, IdentityResolverBox, PrivateIdentityResolver, PrivateIdentityResolverBox};
use futures::{pin_mut, Stream, StreamExt};
use libipld::DefaultParams;
use libp2p::{
	gossipsub, identify,
	identity::Keypair,
	mdns::{self, tokio::Behaviour as MdnsBehaviour},
	multiaddr::Protocol,
	ping,
	swarm::{NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use rxrust::prelude::*;
use std::{sync::Arc, task::Poll, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

pub type EventsSubject<E> = SubjectThreads<Arc<SwarmEvent<E>>, ()>;

pub struct Libp2pNetwork {
	config: Libp2pNetworkConfig,
	shutdown: CancellationToken,
	tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<Behaviour, Context>>,
	events: EventsSubject<NetworkEvent>,
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
		// let kademlia_config: KademliaConfig = Default::default();
		let gossipsub_config = gossipsub::ConfigBuilder::default()
			.max_transmit_size(256 * 1024)
			.build()
			.expect("valid config");

		let behaviour = Behaviour {
			identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
				"/ipfs/0.1.0".into(),
				config.keypair.public(),
			)),
			ping: ping::Behaviour::new(ping::Config::new()),
			mdns: MdnsBehaviour::new(mdns::Config::default(), local_peer_id.clone())?,
			// kad: Kademlia::with_config(local_peer_id.clone(), MemoryStore::new(local_peer_id.clone()),
			// kademlia_config),
			bitswap: Bitswap::new(Default::default(), BitswapStoreClient::new(bitswap), {
				let bitswap_identifier = identifier.clone();
				Box::new(move |t| {
					tokio::spawn(async move {
						t.instrument(tracing::trace_span!("bitswap", application = bitswap_identifier))
							.await
					});
				})
			}),
			gossipsub: gossipsub::Behaviour::new(
				gossipsub::MessageAuthenticity::Signed(config.keypair.clone()),
				gossipsub_config,
			)
			.map_err(|err| anyhow!("gossip failed: {}", err))?,
			didcomm: didcomm::Behaviour::new(resolver.clone(), private_resolver, didcomm::Config { auto_dail: false }),
		};

		// // kad
		// for (peer, address) in config.bootstap.iter() {
		// 	behaviour.kad.add_address(peer, address.clone());
		// }
		// set_network_mode(&mut behaviour, config.mode);
		// if let Err(err) = behaviour.kad.bootstrap() {
		// 	tracing::warn!(?err, "kad-bootstrap-failed");
		// }

		// swarm
		let mut swarm = SwarmBuilder::with_existing_identity(config.keypair.clone())
			.with_tokio()
			.with_quic()
			.with_behaviour(|_| behaviour)?
			.with_swarm_config(|config| config.with_idle_connection_timeout(Duration::from_secs(30)))
			.build();

		// context
		let context = Context {
			discovery: discovery::DiscoveryState::new(resolver.clone(), local_peer_id, Duration::from_secs(30), None),
			heads: heads::HeadsState::new(),
		};

		// tasks
		let (tasks_tx, tasks_rx) = tokio::sync::mpsc::unbounded_channel();

		// events
		let events = SubjectThreads::default();

		// runtime
		let shutdown = CancellationToken::new();
		let mut runtime = Runtime::new(config.clone(), events.clone(), shutdown.child_token());

		// listen
		runtime.listen(swarm.listen_on(config.addr.clone().unwrap_or("/ip4/0.0.0.0/udp/0/quic-v1".parse()?))?);

		// run
		let handle = tokio::runtime::Handle::current().clone();
		tokio::task::spawn_blocking(move || {
			handle.block_on(
				run(swarm, context, runtime, tokio_stream::wrappers::UnboundedReceiverStream::new(tasks_rx))
					.instrument(tracing::trace_span!("network", application = identifier)),
			);
		});

		// result
		Ok(Self { config, shutdown, tasks: tasks_tx, events })
	}

	pub fn spawner(&self) -> TokioNetworkTaskSpawner<Behaviour, Context> {
		TokioNetworkTaskSpawner { tasks: self.tasks.clone() }
	}

	/// Token to gracefully shutdown the network stack.
	/// This will stop accepting new connections and waits until established connections are done.
	pub fn shutdown(&self) -> Shutdown {
		Shutdown { shutdown: self.shutdown.clone() }
	}

	/// Swarm events subject.
	pub fn events(&self) -> EventsSubject<NetworkEvent> {
		self.events.clone()
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
	pub addr: Option<Multiaddr>,
	pub bootstap: Vec<(PeerId, Multiaddr)>,

	/// Network mode to optimize for.
	/// This may change dynamically.
	/// For example when a mobile device gets plugged in to an power outlet.
	pub mode: NetworkMode,
}
impl Libp2pNetworkConfig {
	pub fn from_keypair(keypair: Keypair) -> Self {
		Self { keypair, addr: Default::default(), bootstap: Default::default(), mode: Default::default() }
	}

	/// Add bootstrap peer.
	/// The multiaddress is required to inclide an address (protocol) and and peer id (p2p).
	pub fn add_bootstrap<'a>(&mut self, bootstap: impl Iterator<Item = &'a Multiaddr>) -> Result<(), Vec<Multiaddr>> {
		let mut failed = Vec::new();
		for multiaddr in bootstap {
			let mut addr = multiaddr.to_owned();
			if let Some(Protocol::P2p(peer_id)) = addr.pop() {
				// let peer_id = PeerId::from_multihash(mh).unwrap();
				self.bootstap.push((peer_id, addr));
			} else {
				failed.push(multiaddr.clone());
			}
		}
		match failed.len() {
			0 => Ok(()),
			_ => Err(failed),
		}
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
	events: EventsSubject<NetworkEvent>,
	/// Tasks which have been executed but waiting for events.
	pending_tasks: Vec<NetworkTaskBox<Behaviour, Context>>,
	shutdown: CancellationToken,
}
impl Runtime {
	fn new(config: Libp2pNetworkConfig, events: EventsSubject<NetworkEvent>, shutdown: CancellationToken) -> Self {
		Self { _config: config, listener_id: None, events, shutdown, pending_tasks: Default::default() }
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
	Heads(heads::Event),
}

#[derive(Debug, Clone)]
pub enum ContextLayerEvent {
	Discovery(discovery::DiscoveryEvent),
	Heads(heads::HeadsEvent),
}

pub struct Context {
	pub discovery: discovery::DiscoveryState<IdentityResolverBox>,
	pub heads: heads::HeadsState,
}
impl LayerBehaviour<Behaviour> for Context {
	type ToSwarm = ContextEvent;
	type ToLayer = ContextLayerEvent;

	fn on_swarm_event(&mut self, event: &SwarmEvent<<Behaviour as NetworkBehaviour>::ToSwarm>) {
		LayerBehaviour::<Behaviour>::on_swarm_event(&mut self.discovery, event);
		LayerBehaviour::<Behaviour>::on_swarm_event(&mut self.heads, event);
	}

	fn on_layer_event(&mut self, swarm: &mut Swarm<Behaviour>, event: Self::ToLayer) -> Option<Self::ToSwarm> {
		match event {
			ContextLayerEvent::Discovery(event) => self
				.discovery
				.on_layer_event(swarm, event)
				.map(|event| ContextEvent::Discovery(event)),
			ContextLayerEvent::Heads(event) => {
				self.heads.on_layer_event(swarm, event).map(|event| ContextEvent::Heads(event))
			},
		}
	}

	fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Self::ToLayer> {
		match LayerBehaviour::<Behaviour>::poll(&mut self.discovery, cx) {
			Poll::Ready(event) => return Poll::Ready(ContextLayerEvent::Discovery(event)),
			Poll::Pending => {},
		}
		match LayerBehaviour::<Behaviour>::poll(&mut self.heads, cx) {
			Poll::Ready(event) => return Poll::Ready(ContextLayerEvent::Heads(event)),
			Poll::Pending => {},
		}
		Poll::Pending
	}
}
impl HeadsLayerBehaviourProvider for Context {
	type Event = NetworkEvent;

	fn heads(&self) -> &heads::HeadsState {
		&self.heads
	}

	fn heads_mut(&mut self) -> &mut heads::HeadsState {
		&mut self.heads
	}

	fn heads_event(event: &Self::Event) -> Option<&heads::Event> {
		match event {
			NetworkEvent::Heads(event) => Some(event),
			_ => None,
		}
	}

	fn into_heads_event(event: Self::Event) -> Result<heads::Event, Self::Event> {
		match event {
			NetworkEvent::Heads(event) => Ok(event),
			event => Err(event),
		}
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
	Heads(heads::Event),
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
			ContextEvent::Heads(e) => NetworkEvent::Heads(e),
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
	pub bitswap: Bitswap<DefaultParams>,
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
	type StoreParams = DefaultParams;

	fn bitswap(&self) -> &Bitswap<DefaultParams> {
		&self.bitswap
	}

	fn bitswap_mut(&mut self) -> &mut Bitswap<DefaultParams> {
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
				// execute
				task.execute(&mut swarm, context_layer.layer_mut());

				// move to pending if not complete
				if !task.is_complete() {
					runtime.pending_tasks.push(task);
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
			// run
			result_event =
				runtime.pending_tasks[task_index].on_swarm_event(swarm, context.layer_mut(), result_event.unwrap());

			// done?
			if runtime.pending_tasks[task_index].is_complete() {
				runtime.pending_tasks.remove(task_index);
				task_index -= 1;
			}

			// event consumed?
			if result_event.is_none() {
				return;
			}

			// next
			task_index += 1;
		}

		// other
		if let Some(event) = result_event {
			runtime.events.next(Arc::new(event));
		}
	}
}

fn is_log(event: &SwarmEvent<NetworkEvent>) -> bool {
	match event {
		SwarmEvent::Behaviour(NetworkEvent::Ping(_)) => false,
		_ => true,
	}
}
