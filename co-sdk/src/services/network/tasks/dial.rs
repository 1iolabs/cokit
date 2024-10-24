use anyhow::anyhow;
use co_network::{NetworkTask, NetworkTaskSpawner};
use futures::channel::oneshot;
use libp2p::{
	swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};

/// Dail and wait for connection to be made or fail.
#[derive(Debug)]
pub struct DialNetworkTask {
	peer_id: PeerId,
	addresses: Vec<Multiaddr>,
	tx: Option<oneshot::Sender<Result<(), anyhow::Error>>>,
}
impl DialNetworkTask {
	pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { peer_id, addresses, tx: Some(tx) }, rx)
	}

	pub async fn dial<B, C, N>(spawner: N, peer_id: PeerId, addresses: Vec<Multiaddr>) -> Result<(), anyhow::Error>
	where
		N: NetworkTaskSpawner<B, C>,
		B: NetworkBehaviour,
	{
		let (task, rx) = Self::new(peer_id, addresses);
		spawner.spawn(task)?;
		rx.await?
	}
}
impl<B, C> NetworkTask<B, C> for DialNetworkTask
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		if swarm.is_connected(&self.peer_id) {
			if let Some(tx) = Option::take(&mut self.tx) {
				tx.send(Ok(())).ok();
			}
		} else {
			let opts = DialOpts::peer_id(self.peer_id).addresses(self.addresses.clone()).build();
			tracing::trace!(?opts, "network-dial");
			if let Err(e) = swarm.dial(opts) {
				if let Some(tx) = Option::take(&mut self.tx) {
					tx.send(Err(e.into())).ok();
				}
			}
		}
	}

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match &event {
			SwarmEvent::ConnectionEstablished {
				peer_id,
				connection_id: _,
				endpoint: _,
				num_established: _,
				concurrent_dial_errors: _,
				established_in: _,
			} => {
				if peer_id == &self.peer_id {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Ok(())).ok();
					}
				}
			},
			SwarmEvent::OutgoingConnectionError { connection_id: _, peer_id, error } => {
				if peer_id == &Some(self.peer_id) {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Err(anyhow!("{:?}", error))).ok();
					}
				}
			},
			_ => {},
		}
		Some(event)
	}

	/// Test if the task is complete and can be removed from the queue.
	/// This will be called only after execute has been called.
	fn is_complete(&mut self) -> bool {
		self.tx.is_none()
	}
}
