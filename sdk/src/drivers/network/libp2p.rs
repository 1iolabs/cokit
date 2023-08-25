use super::Network;
use futures::{channel::oneshot, StreamExt};
use libp2p::{
	gossipsub, identify,
	identity::Keypair,
	mdns,
	mdns::tokio::Behaviour as MdnsBehaviour,
	ping,
	swarm::{NetworkBehaviour, SwarmEvent},
	tokio_development_transport, Multiaddr, PeerId, Swarm,
};

pub struct Libp2pNetwork {
	config: Libp2pNetworkConfig,
	shutdown: Option<oneshot::Sender<()>>,
}

#[derive(Clone, Debug)]
pub struct Libp2pNetworkConfig {
	pub keypair: Keypair,
	pub addr: Option<Multiaddr>,
	pub bootstap: Vec<Multiaddr>,
}

impl Network for Libp2pNetwork {
	fn shutdown(mut self) {
		match self.shutdown.take() {
			Some(i) => i.send(()).unwrap(),
			None => {},
		}
	}
}

impl Libp2pNetwork {
	pub async fn new(config: Libp2pNetworkConfig) -> anyhow::Result<Libp2pNetwork> {
		let local_peer_id = PeerId::from(config.keypair.public().clone());
		let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
			.max_transmit_size(262144)
			.build()
			.expect("valid config");
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
			mdns: MdnsBehaviour::new(mdns::Config::default())?,
		};
		let transport = tokio_development_transport(config.keypair.clone())?;
		let mut swarm = Swarm::with_tokio_executor(transport, behaviour, local_peer_id);

		// listen
		swarm.listen_on(config.addr.clone().unwrap_or("/ip4/0.0.0.0/tcp/0".parse()?))?;

		// run
		let (shutdown_tx, shutdown_rx) = oneshot::channel();
		let run_config = config.clone();
		let handle = tokio::runtime::Handle::current().clone();
		tokio::task::spawn_blocking(move || {
			handle.block_on(run(swarm, run_config, shutdown_rx));
		});

		// result
		Ok(Self { config, shutdown: Some(shutdown_tx) })
	}
}

async fn run(mut swarm: Swarm<Behaviour>, config: Libp2pNetworkConfig, mut shutdown: oneshot::Receiver<()>) {
	// log
	tracing::info!("network-running");

	// handle
	while match shutdown.try_recv() {
		Ok(None) => true,     // run not received value
		Ok(Some(_)) => false, // shutdown when received value
		Err(_) => false,      // shutdown when dropped
	} {
		run_once(&mut swarm).await;
	}

	// log
	tracing::info!("network-shutdown");
}

async fn run_once(swarm: &mut Swarm<Behaviour>) {
	match swarm.select_next_some().await {
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
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event")]
struct Behaviour {
	gossipsub: gossipsub::Gossipsub,
	identify: identify::Behaviour,
	ping: ping::Behaviour,
	mdns: MdnsBehaviour,
}

#[derive(Debug)]
enum Event {
	Gossipsub(gossipsub::GossipsubEvent),
	Identify(identify::Event),
	Ping(ping::Event),
	Mdns(mdns::Event),
}

impl From<gossipsub::GossipsubEvent> for Event {
	fn from(event: gossipsub::GossipsubEvent) -> Self {
		Self::Gossipsub(event)
	}
}

impl From<identify::Event> for Event {
	fn from(event: identify::Event) -> Self {
		Self::Identify(event)
	}
}

impl From<ping::Event> for Event {
	fn from(event: ping::Event) -> Self {
		Self::Ping(event)
	}
}

impl From<mdns::Event> for Event {
	fn from(event: mdns::Event) -> Self {
		Self::Mdns(event)
	}
}
