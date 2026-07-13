// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	bitswap::{BitswapMessage, BitswapStoreClient},
	didcomm,
	library::find_peer_id::try_peer_id,
	types::network_task::{NetworkTaskBox, NetworkTaskState, TokioNetworkTaskSpawner},
	NetworkSettings,
};
use anyhow::{anyhow, Context as _};
use co_actor::{time, ActorHandle, TaskSpawner};
use co_identity::{IdentityResolverBox, PrivateIdentityResolverBox};
use futures::{pin_mut, FutureExt, Stream, StreamExt};
#[cfg(feature = "native")]
use libp2p::mdns::{self, tokio::Behaviour as MdnsBehaviour};
use libp2p::{
	autonat, dcutr, gossipsub, identify,
	identity::Keypair,
	noise, ping, relay,
	swarm::{behaviour::toggle::Toggle, dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	yamux, PeerId, StreamProtocol, Swarm, SwarmBuilder,
};
use libp2p_bitswap::{Bitswap, BitswapConfig};
use rand::rngs::OsRng;
use std::{cmp::min, future::Future, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Span};

/// Live-refreshing DNS resolver used by the native DNS transport.
#[cfg(feature = "native")]
type BuildDns = crate::dns::AutoResolver;
/// WASM has no DNS transport
#[cfg(not(feature = "native"))]
type BuildDns = ();

pub const CO_AGENT: &str = "co/0.1.0";
pub const IPFS_IDENTIFY_PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/ipfs/id/1.0.0");

/// Upper bound for actively draining connections during a graceful network shutdown.
const NETWORK_SHUTDOWN_DRAIN: Duration = Duration::from_millis(1000);

pub struct Libp2pNetworkContext {
	pub identifier: String,
	pub tasks: TaskSpawner,
	pub resolver: IdentityResolverBox,
	pub private_resolver: PrivateIdentityResolverBox,
	pub bitswap: ActorHandle<BitswapMessage>,
	/// Maximum accepted bitswap block size in bytes.
	pub max_block_size: usize,
}

pub struct Libp2pNetwork {
	shutdown: CancellationToken,
	tasks: tokio::sync::mpsc::UnboundedSender<NetworkTaskBox<Behaviour>>,
}
impl Libp2pNetwork {
	#[allow(clippy::too_many_arguments)]
	pub async fn new(
		context: Libp2pNetworkContext,
		keypair: Keypair,
		config: NetworkSettings,
		dns: BuildDns,
	) -> anyhow::Result<Libp2pNetwork> {
		// swarm
		// DNS can no longer fail the build: the AutoResolver picks system config with a
		// static fallback and live-refreshes (see crate::dns). BLE/mDNS/loopback come up
		// regardless of adapter state.
		let (local_peer_id, mut swarm) = build_swarm(&context, &keypair, &config, dns).boxed().await?;

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

			// dial bootstrap
			swarm
				.dial(DialOpts::peer_id(peer_id).addresses(vec![bootstrap.clone()]).build())
				.with_context(|| format!("dial bootstrap: {:?}", bootstrap))?;

			// use as explicit gossip peer
			swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
		}

		// tasks
		let (tasks_tx, tasks_rx) = tokio::sync::mpsc::unbounded_channel();

		// runtime
		let shutdown = CancellationToken::new();
		let runtime = Runtime::new(shutdown.child_token());

		// run
		context.tasks.spawn(async move {
			run(&mut swarm, runtime, tokio_stream::wrappers::UnboundedReceiverStream::new(tasks_rx))
				.instrument(tracing::trace_span!("network", application = context.identifier))
				.await;
		});

		// result
		Ok(Self { shutdown, tasks: tasks_tx })
	}

	pub fn spawner(&self) -> TokioNetworkTaskSpawner<Behaviour> {
		TokioNetworkTaskSpawner { tasks: self.tasks.clone() }
	}

	/// Token to gracefully shutdown the network stack.
	/// Stops accepting new connections, actively disconnects established connections, and drains
	/// remaining events for up to `NETWORK_SHUTDOWN_DRAIN` before the run loop exits.
	pub fn shutdown(&self) -> Shutdown {
		Shutdown { shutdown: self.shutdown.clone() }
	}
}

/// Build swarm.
///
/// # Clippy
/// - `clippy::large_stack_frames`: Building the swam is large on stack therefore we return it boxed and call this
///   function boxed.
#[allow(clippy::large_stack_frames)]
async fn build_swarm(
	context: &Libp2pNetworkContext,
	keypair: &Keypair,
	config: &NetworkSettings,
	dns: BuildDns,
) -> Result<(PeerId, Box<Swarm<Behaviour>>), anyhow::Error> {
	let local_peer_id = PeerId::from(keypair.public().clone());

	// behaviour
	let mut behaviour = build_behaviour(context, keypair, config, local_peer_id)?;

	// swarm helper
	macro_rules! build_swarm {
		(@finalize $builder:expr) => {
			if config.nat {
				$builder
					.with_relay_client(noise::Config::new, yamux::Config::default)?
					.with_behaviour(move |_keypair, relay_client| {
						behaviour.relay_client = Some(relay_client).into();
						behaviour
					})
					.context("behaviour")?
					.with_swarm_config(|c| c.with_idle_connection_timeout(config.keep_alive))
					.build()
			} else {
				$builder
					.with_behaviour(move |_keypair| behaviour)?
					.with_swarm_config(|c| c.with_idle_connection_timeout(config.keep_alive))
					.build()
			}
		};
		(@websocket $builder:expr, $dns:expr) => {
			if config.websocket {
				let dns = $dns.clone();
				let b = $builder
					.with_other_transport(move |keypair| {
						// ws(dns(tcp)): DNS inside ws so hostnames reach the ws layer
						// intact (wss certificate semantics) - upstream composition.
						let tcp = libp2p::tcp::tokio::Transport::new(libp2p::tcp::Config::default());
						let dns_tcp = crate::dns::Transport::with_resolver(tcp, dns);
						// `Box<dyn Error>`: the only fallible closure-result type accepted by
						// `with_other_transport`'s sealed `TryIntoTransport` (anyhow is not).
						Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
							libp2p_websocket::Config::new(dns_tcp)
								.upgrade(libp2p::core::upgrade::Version::V1Lazy)
								.authenticate(noise::Config::new(keypair)?)
								.multiplex(yamux::Config::default())
								.map(|(peer, muxer), _| (peer, libp2p::core::muxing::StreamMuxerBox::new(muxer)))
								.boxed(),
						)
					})
					.context("websocket")?;
				build_swarm!(@dns b, $dns)
			} else {
				build_swarm!(@dns $builder, $dns)
			}
		};
		(@dns $builder:expr, $dns:expr) => {{
			let dns = $dns.clone();
			let b = $builder
				.with_other_transport(move |keypair| {
					// dns(tcp or quic): dial-only stack claiming /dns*/ addresses the
					// primary (listening) tcp/quic transports reject. The vendored dns
					// transport claims EVERY address containing a /dns* component without
					// falling through (inner rejection surfaces only asynchronously), so it
					// must be registered after the websocket stack or it would starve ws of
					// dns-hosted ws/wss dials.
					let tcp = libp2p::tcp::tokio::Transport::new(libp2p::tcp::Config::default())
						.upgrade(libp2p::core::upgrade::Version::V1Lazy)
						.authenticate(noise::Config::new(keypair)?)
						.multiplex(yamux::Config::default())
						.map(|(peer, muxer), _| (peer, libp2p::core::muxing::StreamMuxerBox::new(muxer)));
					let quic = libp2p::quic::tokio::Transport::new(libp2p::quic::Config::new(keypair))
						.map(|(peer, muxer), _| (peer, libp2p::core::muxing::StreamMuxerBox::new(muxer)));
					let tcp_or_quic = tcp.or_transport(quic).map(|either, _| either.into_inner());
					// `Box<dyn Error>`: see the websocket arm.
					Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
						crate::dns::Transport::with_resolver(tcp_or_quic, dns).boxed(),
					)
				})
				.context("dns")?;
			build_swarm!(@finalize b)
		}};
	}

	// swarm: native
	#[cfg(feature = "native")]
	let swarm = {
		use libp2p::core::transport::Transport as _;
		let swarm_builder = SwarmBuilder::with_existing_identity(keypair.clone())
			.with_tokio()
			.with_tcp(libp2p::tcp::Config::default(), noise::Config::new, yamux::Config::default)
			.context("tcp")?
			.with_quic();
		build_swarm!(@websocket swarm_builder, dns)
	};

	// swarm: webrtc
	#[cfg(all(feature = "js", target_arch = "wasm32"))]
	let swarm = {
		use libp2p::core::{upgrade::Version, Transport};
		let swarm_builder = SwarmBuilder::with_existing_identity(keypair.clone())
			.with_wasm_bindgen()
			.with_other_transport(|keypair| {
				Ok(libp2p::websocket_websys::Transport::default()
					.upgrade(Version::V1Lazy)
					.authenticate(noise::Config::new(&keypair).expect("noise config"))
					.multiplex(yamux::Config::default())
					.boxed())
			})
			.context("webrtc")?
			.with_other_transport(|keypair| {
				libp2p::webrtc_websys::Transport::new(libp2p::webrtc_websys::Config::new(&keypair))
			})?;
		build_swarm!(@finalize swarm_builder)
	};

	#[cfg(not(feature = "native"))]
	let _ = dns;

	// result
	Ok((local_peer_id, Box::new(swarm)))
}

fn build_behaviour(
	context: &Libp2pNetworkContext,
	keypair: &Keypair,
	config: &NetworkSettings,
	local_peer_id: PeerId,
) -> Result<Behaviour, anyhow::Error> {
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
	let bitswap_config = BitswapConfig { max_block_size: context.max_block_size, ..BitswapConfig::new() };
	let bitswap = Bitswap::new(bitswap_config, BitswapStoreClient::new(context.bitswap.clone()), {
		let bitswap_identifier = context.identifier.to_owned();
		let tasks = context.tasks.clone();
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
	//  we push our updated listen addresses to connected peers when they change so we can use other transports too
	let identify_config = identify::Config::new(IPFS_IDENTIFY_PROTOCOL_NAME.to_string(), keypair.public())
		.with_agent_version(CO_AGENT.into())
		.with_push_listen_addr_updates(true);
	let identify = identify::Behaviour::new(identify_config);

	// mdns
	#[cfg(feature = "native")]
	let mdns: Toggle<MdnsBehaviour> =
		if config.mdns { Some(MdnsBehaviour::new(mdns::Config::default(), local_peer_id)?) } else { None }.into();

	// didcomm
	let didcomm = didcomm::Behaviour::new(
		context.resolver.clone(),
		context.private_resolver.clone(),
		didcomm::Config { auto_dail: false },
	);

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
	let behaviour = Behaviour {
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

	// result
	Ok(behaviour)
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
	/// Tasks which have been executed but waiting for events.
	pending_tasks: Vec<(NetworkTaskBox<Behaviour>, Span)>,
	shutdown: CancellationToken,
	next_delayed_task: Option<time::Instant>,
}
impl Runtime {
	fn new(shutdown: CancellationToken) -> Self {
		Self { shutdown, pending_tasks: Default::default(), next_delayed_task: Default::default() }
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

#[derive(NetworkBehaviour)]
// #[behaviour(to_swarm = "NetworkEvent")]
pub struct Behaviour {
	pub didcomm: didcomm::Behaviour,
	pub gossipsub: gossipsub::Behaviour,
	pub identify: identify::Behaviour,
	#[cfg(feature = "native")]
	pub mdns: Toggle<MdnsBehaviour>,
	pub ping: ping::Behaviour,
	// pub kad: Kademlia<MemoryStore>,
	pub bitswap: Bitswap,
	pub dcutr: Toggle<dcutr::Behaviour>,
	pub relay_server: Toggle<relay::Behaviour>,
	pub relay_client: Toggle<relay::client::Behaviour>,
	pub autonat_server: Toggle<autonat::v2::server::Behaviour>,
	pub autonat_client: Toggle<autonat::v2::client::Behaviour>,
}

pub type NetworkEvent = BehaviourEvent;

async fn run(swarm: &mut Swarm<Behaviour>, mut runtime: Runtime, tasks: impl Stream<Item = NetworkTaskBox<Behaviour>>) {
	// log
	tracing::info!("network-running");

	// handle
	let shutdown = runtime.shutdown.child_token();
	let tasks = tasks.fuse();
	pin_mut!(tasks);
	while runtime.is_running() {
		::tokio::select! {
			// to not stack them up before creating new work
			// use biased as we always want to handle events first
			biased;

			// events
			_ = run_once(swarm, &mut runtime) => {}

			// tasks
			Some(mut task) = tasks.next(), if !tasks.is_done() => {
				let task_span = tracing::trace_span!("network-task", ?task);

				// execute
				let task_state = task_span.in_scope(|| {
					task.execute(swarm);
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

			// shutdown: wake so the loop re-checks is_running() and exits into the drain phase
			_ = shutdown.cancelled() => {}
		}
	}

	// graceful drain: actively close every connection, then pump events until all connections
	// are gone or the drain deadline elapses, so shutdown never waits on the remote to close.
	let peers: Vec<PeerId> = swarm.connected_peers().copied().collect();
	tracing::info!(peers = peers.len(), "network-draining");
	for peer in peers {
		let _ = swarm.disconnect_peer_id(peer);
	}
	let deadline = time::Instant::now() + NETWORK_SHUTDOWN_DRAIN;
	while swarm.connected_peers().next().is_some() && time::Instant::now() < deadline {
		::tokio::select! {
			biased;
			_ = run_once(swarm, &mut runtime) => {}
			_ = time::sleep_until(deadline) => {}
		}
	}

	// log
	tracing::info!("network-shutdown");
}

async fn run_once(swarm: &mut Swarm<Behaviour>, runtime: &mut Runtime) {
	// event
	let network_event: Option<SwarmEvent<NetworkEvent>> = ::tokio::select! {
		swarm_event = swarm.select_next_some() => {
			Some(swarm_event)
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

		// recovery diagnostics: surface listener death prominently
		match &event {
			SwarmEvent::ListenerClosed { listener_id, addresses, reason } => {
				tracing::info!(?listener_id, ?addresses, ?reason, "network-listener-closed");
			},
			SwarmEvent::ListenerError { listener_id, error } => {
				tracing::info!(?listener_id, ?error, "network-listener-error");
			},
			_ => {},
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
				result_event = task.on_swarm_event(swarm, result_event.unwrap());

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
						task.execute(swarm);

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
