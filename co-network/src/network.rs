// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	bitswap::{BitswapMessage, BitswapStoreClient},
	didcomm, discovery,
	library::find_peer_id::try_peer_id,
	types::{
		layer_behaviour::{Layer, LayerBehaviour},
		network_task::{NetworkTaskBox, NetworkTaskState, TokioNetworkTaskSpawner},
	},
	NetworkSettings,
};
use anyhow::anyhow;
use co_actor::{time, ActorHandle, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use co_primitives::DynamicCoDate;
use futures::{pin_mut, Stream, StreamExt};
#[cfg(feature = "native")]
use libp2p::mdns::{self, tokio::Behaviour as MdnsBehaviour};
use libp2p::{
	autonat, dcutr, gossipsub, identify,
	identity::Keypair,
	noise, ping, relay,
	swarm::{behaviour::toggle::Toggle, dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	yamux, PeerId, StreamProtocol, Swarm, SwarmBuilder,
};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use rand::rngs::OsRng;
use std::{cmp::min, future::Future, task::Poll, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Span};
pub const CO_AGENT: &str = "co/0.1.0";
pub const IPFS_IDENTIFY_PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/ipfs/id/1.0.0");

pub struct Libp2pNetwork {
	shutdown: CancellationToken,
	tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<Behaviour, Context>>,
}
impl Libp2pNetwork {
	#[allow(clippy::too_many_arguments)]
	pub async fn new(
		identifier: String,
		keypair: Keypair,
		config: NetworkSettings,
		date: DynamicCoDate,
		tasks: TaskSpawner,
		resolver: IdentityResolverBox,
		private_resolver: PrivateIdentityResolverBox,
		bitswap: ActorHandle<BitswapMessage>,
	) -> anyhow::Result<Libp2pNetwork> {
		let resolver = IdentityResolverBox::new(resolver);
		let private_resolver = PrivateIdentityResolverBox::new(private_resolver);
		let local_peer_id = PeerId::from(keypair.public().clone());

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
			gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair.clone()), gossipsub_config)
				.map_err(|err| anyhow!("gossip failed: {}", err))?;

		// bitswap
		let bitswap = Bitswap::<libipld::DefaultParams>::new(Default::default(), BitswapStoreClient::new(bitswap), {
			let bitswap_identifier = identifier.clone();
			let tasks = tasks.clone();
			Box::new(move |t| {
				tasks.spawn(async move {
					t.instrument(tracing::trace_span!("bitswap", application = bitswap_identifier))
						.await
				});
			})
		});

		// relay
		let relay_server = if config.relay {
			let mut relay_config = relay::Config::default();
			if let Some(bytes) = config.max_circuit_bytes {
				relay_config.max_circuit_bytes = bytes;
			}
			if let Some(duration) = config.max_circuit_duration {
				relay_config.max_circuit_duration = duration;
			}
			Some(libp2p::relay::Behaviour::new(local_peer_id, relay_config))
		} else {
			None
		}
		.into();

		// identify
		let identify_config = identify::Config::new(IPFS_IDENTIFY_PROTOCOL_NAME.to_string(), keypair.public())
			.with_agent_version(CO_AGENT.into());
		let identify = identify::Behaviour::new(identify_config);

		// mdns
		#[cfg(feature = "native")]
		let mdns: Toggle<MdnsBehaviour> =
			if config.mdns { Some(MdnsBehaviour::new(mdns::Config::default(), local_peer_id)?) } else { None }.into();

		// didcomm
		let didcomm = didcomm::Behaviour::new(resolver.clone(), private_resolver, didcomm::Config { auto_dail: false });

		// autonat
		let autonat_server = if config.relay { Some(autonat::v2::server::Behaviour::new(OsRng)) } else { None }.into();
		let autonat_client = if config.nat {
			Some(autonat::v2::client::Behaviour::new(OsRng, autonat::v2::client::Config::default()))
		} else {
			None
		}
		.into();

		// dcutr
		let dcutr = if config.nat { Some(dcutr::Behaviour::new(local_peer_id)) } else { None }.into();

		// behaviour
		let mut behaviour = Behaviour {
			identify,
			ping: ping::Behaviour::new(ping::Config::new()),
			#[cfg(feature = "native")]
			mdns,
			// kad,
			bitswap,
			gossipsub,
			didcomm,
			dcutr,
			relay_server,
			relay_client: None.into(),
			autonat_server,
			autonat_client,
		};

		// swarm
		#[cfg(feature = "native")]
		let mut swarm = {
			let swarm_builder = SwarmBuilder::with_existing_identity(keypair.clone())
				.with_tokio()
				.with_tcp(libp2p::tcp::Config::default(), noise::Config::new, yamux::Config::default)?
				.with_quic()
				.with_dns()?
				.with_websocket(noise::Config::new, yamux::Config::default)
				.await
				.map_err(|e| anyhow!("websocket transport: {e}"))?;
			if config.nat {
				swarm_builder
					.with_relay_client(noise::Config::new, yamux::Config::default)?
					.with_behaviour(move |_keypair, relay_client| {
						behaviour.relay_client = Some(relay_client).into();
						behaviour
					})?
					.with_swarm_config(|swarm_config| swarm_config.with_idle_connection_timeout(config.keep_alive))
					.build()
			} else {
				swarm_builder
					.with_behaviour(move |_keypair| behaviour)?
					.with_swarm_config(|swarm_config| swarm_config.with_idle_connection_timeout(config.keep_alive))
					.build()
			}
		};

		#[cfg(all(feature = "js", target_arch = "wasm32"))]
		let mut swarm = {
			use libp2p::core::{upgrade::Version, Transport};
			let swarm_builder = SwarmBuilder::with_existing_identity(keypair.clone())
				.with_wasm_bindgen()
				.with_other_transport(|keypair| {
					Ok(libp2p::websocket_websys::Transport::default()
						.upgrade(Version::V1Lazy)
						.authenticate(noise::Config::new(&keypair).expect("noise config"))
						.multiplex(yamux::Config::default())
						.boxed())
				})?
				.with_other_transport(|keypair| {
					libp2p::webrtc_websys::Transport::new(libp2p::webrtc_websys::Config::new(&keypair))
				})?;
			if config.nat {
				swarm_builder
					.with_relay_client(noise::Config::new, yamux::Config::default)?
					.with_behaviour(move |_keypair, relay_client| {
						behaviour.relay_client = Some(relay_client).into();
						behaviour
					})?
					.with_swarm_config(|swarm_config| swarm_config.with_idle_connection_timeout(config.keep_alive))
					.build()
			} else {
				swarm_builder
					.with_behaviour(move |_keypair| behaviour)?
					.with_swarm_config(|swarm_config| swarm_config.with_idle_connection_timeout(config.keep_alive))
					.build()
			}
		};

		// external addresses
		for external_address in config.external_addresses.iter() {
			swarm.add_external_address(external_address.clone());
		}

		// bootstrap
		for bootstrap in config.bootstrap.iter() {
			let peer_id = try_peer_id(bootstrap)?;
			if local_peer_id == peer_id {
				continue;
			}

			// listen on bootstrap as relay
			if config.nat {
				swarm.listen_on(bootstrap.clone().with(multiaddr::Protocol::P2pCircuit)).ok();
			}

			// dial bootstrap
			swarm.dial(DialOpts::peer_id(peer_id).addresses(vec![bootstrap.clone()]).build())?;

			// use as explicent gossip peer
			swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
		}

		// context
		let context = Context {
			discovery: discovery::DiscoveryState::new(
				date.clone(),
				resolver.clone(),
				local_peer_id,
				Duration::from_secs(30),
				None,
			),
		};

		// tasks
		let (tasks_tx, tasks_rx) = tokio::sync::mpsc::unbounded_channel();

		// runtime
		let shutdown = CancellationToken::new();
		let runtime = Runtime::new(shutdown.child_token());

		// listen (browsers connect via relay, not direct listen)
		#[cfg(not(target_arch = "wasm32"))]
		let runtime = {
			let mut runtime = runtime;
			runtime.listen(swarm.listen_on(config.listen)?);
			runtime
		};

		// run
		tasks.spawn(async move {
			run(swarm, context, runtime, tokio_stream::wrappers::UnboundedReceiverStream::new(tasks_rx))
				.instrument(tracing::trace_span!("network", application = identifier))
				.await;
		});

		// result
		Ok(Self { shutdown, tasks: tasks_tx })
	}

	pub fn spawner(&self) -> TokioNetworkTaskSpawner<Behaviour, Context> {
		TokioNetworkTaskSpawner { tasks: self.tasks.clone() }
	}

	/// Token to gracefully shutdown the network stack.
	/// This will stop accepting new connections and waits until established connections are done.
	pub fn shutdown(&self) -> Shutdown {
		Shutdown { shutdown: self.shutdown.clone() }
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

struct Runtime {
	#[cfg(not(target_arch = "wasm32"))]
	listener_id: Option<libp2p::core::transport::ListenerId>,
	/// Tasks which have been executed but waiting for events.
	pending_tasks: Vec<(NetworkTaskBox<Behaviour, Context>, Span)>,
	shutdown: CancellationToken,
	next_delayed_task: Option<time::Instant>,
}
impl Runtime {
	fn new(shutdown: CancellationToken) -> Self {
		Self {
			#[cfg(not(target_arch = "wasm32"))]
			listener_id: None,
			shutdown,
			pending_tasks: Default::default(),
			next_delayed_task: Default::default(),
		}
	}

	#[cfg(not(target_arch = "wasm32"))]
	fn listen(&mut self, id: libp2p::core::transport::ListenerId) {
		self.listener_id = Some(id);
	}

	fn is_running(&self) -> bool {
		!self.shutdown.is_cancelled()
	}

	fn use_task_state(&mut self, state: NetworkTaskState) {
		if let NetworkTaskState::Delayed(until) = state {
			self.next_delayed_task = Some(match self.next_delayed_task {
				Some(next_delayed_task) => min(next_delayed_task, until),
				None => until,
			});
		}
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
			ContextLayerEvent::Discovery(event) => {
				self.discovery.on_layer_event(swarm, event).map(ContextEvent::Discovery)
			},
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

#[allow(unused)]
#[derive(Debug, derive_more::From)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum NetworkEvent {
	Didcomm(didcomm::Event),
	Gossipsub(gossipsub::Event),
	Identify(identify::Event),
	#[cfg(feature = "native")]
	Mdns(libp2p::mdns::Event),
	Ping(ping::Event),
	// Kad(kad::Event),
	Bitswap(BitswapEvent),
	Discovery(discovery::Event),
	Dcutr(dcutr::Event),
	RelayServer(relay::Event),
	RelayClient(relay::client::Event),
	AutonatServer(autonat::v2::server::Event),
	AutonatClient(autonat::v2::client::Event),
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
	#[cfg(feature = "native")]
	pub mdns: Toggle<MdnsBehaviour>,
	pub ping: ping::Behaviour,
	// pub kad: Kademlia<MemoryStore>,
	pub bitswap: Bitswap<libipld::DefaultParams>,
	pub dcutr: Toggle<dcutr::Behaviour>,
	pub relay_server: Toggle<relay::Behaviour>,
	pub relay_client: Toggle<relay::client::Behaviour>,
	pub autonat_server: Toggle<autonat::v2::server::Behaviour>,
	pub autonat_client: Toggle<autonat::v2::client::Behaviour>,
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

	#[cfg(feature = "native")]
	fn mdns_mut(&mut self) -> Option<&mut libp2p::mdns::tokio::Behaviour> {
		self.mdns.as_mut()
	}

	#[cfg(feature = "native")]
	fn mdns_event(event: &<Self as NetworkBehaviour>::ToSwarm) -> Option<&libp2p::mdns::Event> {
		match event {
			NetworkEvent::Mdns(e) => Some(e),
			_ => None,
		}
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
		::tokio::select! {
			// to not stack them up before creating new work
			// use biased as we always want to handle events first
			biased;

			// events
			_ = run_once(&mut swarm, &mut context_layer, &mut runtime) => {}

			// tasks
			Some(mut task) = tasks.next(), if !tasks.is_done() => {
				let task_span = tracing::trace_span!("network-task", ?task);

				// execute
				let task_state = task_span.in_scope(|| {
					task.execute(&mut swarm, context_layer.layer_mut());
					task.task_state()
				});

				// move to pending if not complete
				if task_state != NetworkTaskState::Complete {
					// log
					task_span.in_scope(|| {
						tracing::trace!("network-task-pending");
					});

					// pending
					runtime.pending_tasks.push((task, task_span));
					runtime.use_task_state(task_state);
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
	let network_event: Option<SwarmEvent<NetworkEvent>> = ::tokio::select! {
		swarm_event = swarm.select_next_some() => {
			context.on_swarm_event(&swarm_event);
			Some(swarm_event)
		},
		layer_event = context.select_next_some() => {
			context.on_layer_event(swarm, layer_event).map(|layer_event| SwarmEvent::Behaviour(NetworkEvent::from(layer_event)))
		},
		Some(_) = option_await(runtime.next_delayed_task.map(time::sleep_until)) => {
			runtime.next_delayed_task = None;
			None
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
			let task_state = {
				let (task, task_span) = &mut runtime.pending_tasks[task_index];
				let _enter = task_span.enter();

				// run
				result_event = task.on_swarm_event(swarm, context.layer_mut(), result_event.unwrap());

				// complete?
				task.task_state()
			};

			// complete?
			run_task_complete(runtime, &mut task_index, task_state);

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
	} else {
		let mut task_index = 0;
		while task_index < runtime.pending_tasks.len() {
			// handle
			let task_state = {
				let (task, task_span) = &mut runtime.pending_tasks[task_index];

				// pending
				let task_state = task.task_state();
				match task_state {
					NetworkTaskState::Pending => {
						let _enter = task_span.enter();

						// run
						task.execute(swarm, context.layer_mut());

						// complete?
						task.task_state()
					},
					task_state => task_state,
				}
			};

			// complete?
			run_task_complete(runtime, &mut task_index, task_state);

			// next
			task_index += 1;
		}
	}
}

fn run_task_complete(runtime: &mut Runtime, task_index: &mut usize, task_state: NetworkTaskState) {
	// use
	runtime.use_task_state(task_state);

	// done?
	if task_state == NetworkTaskState::Complete {
		// remove
		let (task, task_span) = runtime.pending_tasks.remove(*task_index);
		*task_index -= 1;

		// log
		task_span.in_scope(|| {
			tracing::trace!(?task, "network-task-completed");
		});
	}
}

fn is_log(event: &SwarmEvent<NetworkEvent>) -> bool {
	!matches!(event, SwarmEvent::Behaviour(NetworkEvent::Ping(_)))
}

async fn option_await<T, O>(t: Option<T>) -> Option<O>
where
	T: Future<Output = O>,
{
	match t {
		Some(fut) => Some(fut.await),
		None => None,
	}
}
