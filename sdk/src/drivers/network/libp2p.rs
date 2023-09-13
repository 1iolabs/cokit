use super::Network;
use co_network::didcomm;
use futures::{channel::oneshot, StreamExt};
use libp2p::{
	gossipsub, identify,
	identity::Keypair,
	mdns,
	mdns::tokio::Behaviour as MdnsBehaviour,
	ping,
	swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
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
		let gossipsub_config = gossipsub::ConfigBuilder::default()
			.max_transmit_size(256 * 1024)
			.build()
			.expect("valid config");
		let didcomm_config: didcomm::Config = didcomm::Config {
			..Default::default(),
			auto_dail: false,
		};
		let behaviour = Behaviour {
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
			mdns: MdnsBehaviour::new(mdns::Config::default() /* , local_peer_id.clone() */)?,
			didcomm: didcomm::Behavior::new(didcomm_config),
		};
		let transport = tokio_development_transport(config.keypair.clone())?;
		let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();

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
	let mut running = true;
	while running {
		tokio::select! {
			_ = shutdown = {
				running = false;
			},
			_ = run_once(&mut swarm) => {},
		}
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
struct Behaviour {
	didcomm: didcomm::Behavior,
	gossipsub: gossipsub::Behaviour,
	identify: identify::Behaviour,
	mdns: MdnsBehaviour,
	ping: ping::Behaviour,
}
