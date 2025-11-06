use anyhow::anyhow;
use co_network::{NetworkTask, NetworkTaskSpawner};
use futures::channel::oneshot;
use libp2p::{
	swarm::{dial_opts::DialOpts, ConnectionId, NetworkBehaviour, SwarmEvent},
	Multiaddr, PeerId, Swarm,
};
use std::mem::take;

/// Dail and wait for connection to be made or fail.
#[derive(Debug)]
pub struct DialNetworkTask {
	opts: Option<DialOpts>,
	complete: ConnectionId,
	tx: Option<oneshot::Sender<Result<PeerId, anyhow::Error>>>,
}
impl DialNetworkTask {
	pub async fn dial<B, C, N>(
		spawner: &N,
		peer_id: Option<PeerId>,
		addresses: Vec<Multiaddr>,
	) -> Result<PeerId, anyhow::Error>
	where
		N: NetworkTaskSpawner<B, C>,
		B: NetworkBehaviour,
	{
		let opts = match peer_id {
			Some(peer_id) => DialOpts::peer_id(peer_id).addresses(addresses).build(),
			None => {
				let address = addresses.into_iter().next().ok_or(anyhow!("Expected exactly one address"))?;
				DialOpts::unknown_peer_id().address(address).build()
			},
		};
		let (tx, rx) = oneshot::channel();
		spawner.spawn(Self { complete: opts.connection_id(), opts: Some(opts), tx: Some(tx) })?;
		rx.await?
	}
}
impl<B, C> NetworkTask<B, C> for DialNetworkTask
where
	B: NetworkBehaviour,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		if let Some(opts) = take(&mut self.opts) {
			// already connected?
			if let Some(peer_id) = &opts.get_peer_id() {
				if swarm.is_connected(peer_id) {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Ok(*peer_id)).ok();
					}
					return;
				}
			}

			// dail
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
				connection_id,
				endpoint: _,
				num_established: _,
				concurrent_dial_errors: _,
				established_in: _,
			} => {
				if connection_id == &self.complete {
					if let Some(tx) = Option::take(&mut self.tx) {
						tx.send(Ok(*peer_id)).ok();
					}
				}
			},
			SwarmEvent::OutgoingConnectionError { connection_id, peer_id: _, error } => {
				if connection_id == &self.complete {
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
