use co_network::didcomm;
use futures::{channel::oneshot, StreamExt};
use libp2p::{
	gossipsub, identify,
	identity::Keypair,
	mdns,
	mdns::tokio::Behaviour as MdnsBehaviour,
	ping,
	swarm::{ConnectionHandler, IntoConnectionHandler, NetworkBehaviour, SwarmBuilder, SwarmEvent},
	tokio_development_transport, Multiaddr, PeerId, Swarm,
};
use rxrust::prelude::*;
use std::sync::Arc;

pub type Task<B> = Box<dyn Send + Fn(&mut Swarm<B>)>;
pub type OnSwarmEvent<B, E> = Box<dyn Send + Fn(&mut Swarm<B>, SwarmEvent<E, THandlerErr<B>>)>;
pub type EventsSubject<B, E> = SubjectThreads<Arc<SwarmEvent<E, THandlerErr<B>>>, ()>;

// pub type BehaviourConnectionHandler<B: NetworkBehaviour> =
// 	<<B as NetworkBehaviour>::ConnectionHandler as IntoConnectionHandler>::Handler;
// pub type BehaviourOutEventType<B: NetworkBehaviour> = <BehaviourConnectionHandler<B> as ConnectionHandler>::OutEvent;
// pub type BehaviourErrorType<B: NetworkBehaviour> = <BehaviourConnectionHandler<B> as ConnectionHandler>::Error;

type THandlerErr<TBehaviour> = <<<TBehaviour as NetworkBehaviour>::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::Error;

pub struct Libp2pNetwork {
	config: Libp2pNetworkConfig,
	shutdown: Option<oneshot::Sender<()>>,
	tasks: tokio::sync::mpsc::UnboundedSender<Task<Behaviour>>,
	events: EventsSubject<Behaviour, BehaviourEvent>,
}

#[derive(Clone, Debug)]
pub struct Libp2pNetworkConfig {
	pub keypair: Keypair,
	pub addr: Option<Multiaddr>,
	pub bootstap: Vec<Multiaddr>,
}

struct Runtime {
	config: Libp2pNetworkConfig,
	shutdown: Option<oneshot::Receiver<()>>,
	listener_id: Option<libp2p::core::transport::ListenerId>,
	tasks: tokio::sync::mpsc::UnboundedReceiver<Task<Behaviour>>,
	// on_swarm_event: OnSwarmEvent<Behaviour>,
	events: SubjectThreads<Arc<SwarmEvent<BehaviourEvent, THandlerErr<Behaviour>>>, ()>,
}
impl Runtime {
	fn new(
		config: Libp2pNetworkConfig,
		shutdown: oneshot::Receiver<()>,
		tasks: tokio::sync::mpsc::UnboundedReceiver<Task<Behaviour>>,
		// on_swarm_event: OnSwarmEvent<Behaviour>,
		events: SubjectThreads<Arc<SwarmEvent<BehaviourEvent, THandlerErr<Behaviour>>>, ()>,
	) -> Self {
		Self { config, shutdown: Some(shutdown), listener_id: None, tasks, events }
	}

	fn listen(&mut self, id: libp2p::core::transport::ListenerId) {
		self.listener_id = Some(id);
	}

	fn is_running(&mut self, swarm: &mut Swarm<Behaviour>) -> bool {
		let running = match &mut self.shutdown {
			None => false,
			Some(r) => match r.try_recv() {
				// dropped or received signal
				Err(_) | Ok(Some(_)) => {
					// stop listening
					if let Some(listener_id) = self.listener_id.take() {
						swarm.remove_listener(listener_id);
					}

					// do not ask again
					self.shutdown = None;

					// not running
					false
				},

				// no signal received yet
				Ok(None) => true,
			},
		};
		running || swarm.connected_peers().peekable().peek().is_some()
	}
}

impl Libp2pNetwork {
	pub async fn new(config: Libp2pNetworkConfig) -> anyhow::Result<Libp2pNetwork> {
		let local_peer_id = PeerId::from(config.keypair.public().clone());
		let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
			.max_transmit_size(256 * 1024)
			.build()
			.expect("valid config");
		let didcomm_config: didcomm::Config = didcomm::Config { auto_dail: false, ..Default::default() };
		let behaviour = Behaviour {
			gossipsub: gossipsub::Gossipsub::new(
				gossipsub::MessageAuthenticity::Signed(config.keypair.clone()),
				gossipsub_config,
			)
			.expect("Valid configuration"),
			identify: libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
				"/ipfs/0.1.0".into(),
				config.keypair.public(),
			)),
			ping: ping::Behaviour::new(ping::Config::new()),
			mdns: MdnsBehaviour::new(mdns::Config::default() /* , local_peer_id.clone() */)?,
			didcomm: didcomm::Behavior::new(didcomm_config),
		};
		let transport = tokio_development_transport(config.keypair.clone())?;
		let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();

		// tasks
		let (tasks_tx, tasks_rx) = tokio::sync::mpsc::unbounded_channel();

		// events
		let events = SubjectThreads::default();

		// runtime
		let (shutdown_tx, shutdown_rx) = oneshot::channel();
		let mut runtime = Runtime::new(config.clone(), shutdown_rx, tasks_rx, events.clone());

		// listen
		runtime.listen(swarm.listen_on(config.addr.clone().unwrap_or("/ip4/0.0.0.0/tcp/0".parse()?))?);

		// run
		let handle = tokio::runtime::Handle::current().clone();
		tokio::task::spawn_blocking(move || {
			handle.block_on(run(swarm, runtime));
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
	pub fn queue_behaviour_task(
		&self,
		task: Task<Behaviour>,
	) -> Result<(), tokio::sync::mpsc::error::SendError<Task<Behaviour>>> {
		self.tasks.send(task)
	}

	/// Swarm events subject.
	pub fn events(&self) -> EventsSubject<Behaviour, BehaviourEvent> {
		self.events.clone()
	}
}

async fn run(mut swarm: Swarm<Behaviour>, mut runtime: Runtime) {
	// log
	tracing::info!("network-running");

	// handle
	while runtime.is_running(&mut swarm) {
		run_once(&mut swarm, &mut runtime).await;
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
			tracing::info!(?event, "network-behaviour-event");
		},
		event => {
			tracing::info!(?event, "network-event");
		},
	}

	// handler
	// TODO: solution without heap? performance?
	runtime.events.next(Arc::new(event));
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
	didcomm: didcomm::Behavior,
	gossipsub: gossipsub::Gossipsub,
	identify: identify::Behaviour,
	mdns: MdnsBehaviour,
	ping: ping::Behaviour,
}
