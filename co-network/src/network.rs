use super::didcomm;
use futures::{channel::oneshot, StreamExt};
use libp2p::{
	gossipsub, identify,
	identity::Keypair,
	kad::{store::MemoryStore, Behaviour as Kademlia, Config as KademliaConfig},
	mdns,
	mdns::tokio::Behaviour as MdnsBehaviour,
	multiaddr::Protocol,
	ping,
	swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use rxrust::prelude::*;
use std::sync::Arc;

pub type Task<B> = Box<dyn Fn(&mut Swarm<B>) + Send>;
pub type EventsSubject<E> = SubjectThreads<Arc<SwarmEvent<E>>, ()>;

pub struct Libp2pNetwork {
	config: Libp2pNetworkConfig,
	shutdown: Option<oneshot::Sender<()>>,
	tasks: tokio::sync::mpsc::UnboundedSender<Task<Behaviour>>,
	events: EventsSubject<BehaviourEvent>,
}
impl Libp2pNetwork {
	pub fn new(config: Libp2pNetworkConfig) -> anyhow::Result<Libp2pNetwork> {
		let local_peer_id = PeerId::from(config.keypair.public().clone());
		let gossipsub_config = gossipsub::ConfigBuilder::default()
			.max_transmit_size(256 * 1024)
			.build()
			.expect("valid config");
		let didcomm_config: didcomm::Config = didcomm::Config { auto_dail: false, ..Default::default() };
		let kademlia_config: KademliaConfig = Default::default();
		let mut behaviour = Behaviour {
			gossipsub: gossipsub::Behaviour::new(
				gossipsub::MessageAuthenticity::Signed(config.keypair.clone()),
				gossipsub_config,
			)
			.expect("Valid configuration"),
			identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
				"/ipfs/0.1.0".into(),
				config.keypair.public(),
			)),
			ping: ping::Behaviour::new(ping::Config::new()),
			mdns: MdnsBehaviour::new(mdns::Config::default(), local_peer_id.clone())?,
			didcomm: didcomm::Behavior::new(didcomm_config),
			kad: Kademlia::with_config(local_peer_id.clone(), MemoryStore::new(local_peer_id.clone()), kademlia_config),
		};

		// kad
		for (peer, address) in config.bootstap.iter() {
			behaviour.kad.add_address(peer, address.clone());
		}
		set_network_mode(&mut behaviour, config.mode);
		if let Err(err) = behaviour.kad.bootstrap() {
			tracing::warn!(?err, "kad-bootstrap-failed");
		}

		// swarm
		let mut swarm = SwarmBuilder::with_existing_identity(config.keypair.clone())
			.with_tokio()
			.with_quic()
			.with_behaviour(|_| behaviour)?
			.build();

		// tasks
		let (tasks_tx, tasks_rx) = tokio::sync::mpsc::unbounded_channel();

		// events
		let events = SubjectThreads::default();

		// runtime
		let (shutdown_tx, shutdown_rx) = oneshot::channel();
		let mut runtime = Runtime::new(config.clone(), events.clone());

		// listen
		runtime.listen(swarm.listen_on(config.addr.clone().unwrap_or("/ip4/0.0.0.0/tcp/0".parse()?))?);

		// run
		let handle = tokio::runtime::Handle::current().clone();
		tokio::task::spawn_blocking(move || {
			handle.block_on(run(swarm, runtime, tasks_rx, shutdown_rx));
		});

		// result
		Ok(Self { config, shutdown: Some(shutdown_tx), tasks: tasks_tx, events })
	}

	/// Gracefully shutdown the network stack.
	/// This will stop accepting new connections and waits until established connections are done.
	pub fn shutdown(&mut self) {
		// trigger shutdown signal
		if let Some(shutdown) = self.shutdown.take() {
			let _ = shutdown.send(());
		}
	}

	/// Sends a task to execute on the behavior to the queue.
	pub fn queue_behaviour_task(&self, task: Task<Behaviour>) -> Result<(), Libp2pNetworkError> {
		self.tasks.send(task)?;
		Ok(())
	}

	/// Swarm events subject.
	pub fn events(&self) -> EventsSubject<BehaviourEvent> {
		self.events.clone()
	}

	pub fn config(&self) -> &Libp2pNetworkConfig {
		&self.config
	}

	/// Change network mode.
	pub fn set_network_mode(&mut self, mode: NetworkMode) -> Result<(), Libp2pNetworkError> {
		if self.config.mode != mode {
			self.config.mode = mode;
			self.queue_behaviour_task(Box::new(move |swarm| {
				set_network_mode(swarm.behaviour_mut(), mode);
			}))?;
		}
		Ok(())
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
	events: EventsSubject<BehaviourEvent>,
	running: bool,
}
impl Runtime {
	fn new(config: Libp2pNetworkConfig, events: EventsSubject<BehaviourEvent>) -> Self {
		Self { _config: config, listener_id: None, events, running: true }
	}

	fn listen(&mut self, id: libp2p::core::transport::ListenerId) {
		self.listener_id = Some(id);
	}

	fn is_running(&self) -> bool {
		self.running
	}
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
	didcomm: didcomm::Behavior,
	gossipsub: gossipsub::Behaviour,
	identify: identify::Behaviour,
	mdns: MdnsBehaviour,
	ping: ping::Behaviour,
	kad: Kademlia<MemoryStore>,
}

#[derive(Debug, thiserror::Error)]
pub enum Libp2pNetworkError {
	#[error("Shutdown in progress. Operation canceled.")]
	Shutdown,
}
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Libp2pNetworkError {
	fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
		Self::Shutdown
	}
}

fn set_network_mode(behaviour: &mut Behaviour, mode: NetworkMode) {
	match mode {
		NetworkMode::Full => behaviour.kad.set_mode(Some(libp2p::kad::Mode::Server)),
		NetworkMode::Light => behaviour.kad.set_mode(Some(libp2p::kad::Mode::Client)),
	}
}

async fn run(
	mut swarm: Swarm<Behaviour>,
	mut runtime: Runtime,
	mut tasks: tokio::sync::mpsc::UnboundedReceiver<Task<Behaviour>>,
	mut shutdown: oneshot::Receiver<()>,
) {
	// log
	tracing::info!("network-running");

	// handle
	while runtime.is_running() || swarm.connected_peers().peekable().peek().is_some() {
		tokio::select! {
			// to not stack them up before creating new work
			// use biased as we always want to handle events first
			biased;

			// events
			_ = run_once(&mut swarm, &mut runtime) => {}

			// tasks
			task = tasks.recv() => {
				if let Some(task) = task {
					task(&mut swarm);
				}
			},

			// shutdown
			_ = &mut shutdown => {
				runtime.running = false;
			}
		}
	}

	// log
	tracing::info!("network-shutdown");
}

async fn run_once(swarm: &mut Swarm<Behaviour>, runtime: &mut Runtime) {
	let event = swarm.select_next_some().await;

	// log
	match &event {
		SwarmEvent::NewListenAddr { address, .. } => {
			tracing::info!(?address, "network-listening");
		},
		SwarmEvent::Behaviour(event) => {
			tracing::debug!(?event, "network-behaviour-event");
		},
		event => {
			tracing::debug!(?event, "network-event");
		},
	}

	// log
	match event {
		SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns_event)) => handle_mdns(swarm, runtime, mdns_event),
		// SwarmEvent::Behaviour(event) => {
		// 	tracing::info!(?event, "network-behaviour-event");
		// },
		event => {
			tracing::info!(?event, "network-event");
			runtime.events.next(Arc::new(event));
		},
	}
}

fn handle_mdns(swarm: &mut Swarm<Behaviour>, _runtime: &mut Runtime, event: mdns::Event) {
	match event {
		mdns::Event::Discovered(list) => {
			tracing::debug!(?list, "mdns::Event::Discovered");

			// use
			for (peer_id, multiaddr) in list {
				match swarm.dial(DialOpts::peer_id(peer_id.clone()).addresses(vec![multiaddr.clone()]).build()) {
					Err(err) => tracing::warn!(?err, ?peer_id, ?multiaddr, "mdns-dail-failed"),
					_ => {},
				}
				//_swarm.behaviour_mut().gossipsub.add_explicit_peer(peer_id);
				//runtime.add_explicit_peer(&mut swarm, &peer_id);
			}
		},
		mdns::Event::Expired(list) => {
			tracing::debug!(?list, "mdns::Event::Expired");

			// use
			for (peer_id, _multiaddr) in list {
				match swarm.disconnect_peer_id(peer_id.clone()) {
					Err(_) => tracing::warn!(?peer_id, "mdnd-disconnect-failed"),
					_ => {},
				}
				//runtime.remove_explicit_peer(&mut swarm, &peer_id);
			}
		},
	}
}
