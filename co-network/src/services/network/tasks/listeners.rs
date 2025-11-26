use crate::{
	network::{Behaviour, Context, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use futures::channel::oneshot;
use libp2p::{swarm::SwarmEvent, Multiaddr, Swarm};
use std::{collections::BTreeSet, mem::take};

/// Get active listener addresses.
/// If no listener is present it will wait for the first to come available.
#[derive(Debug)]
pub struct ListnersNetworkTask {
	local: bool,
	external: bool,
	result: Option<oneshot::Sender<BTreeSet<Multiaddr>>>,
}
impl ListnersNetworkTask {
	pub async fn listeners(
		spawner: &CoNetworkTaskSpawner,
		local: bool,
		external: bool,
	) -> Result<BTreeSet<Multiaddr>, anyhow::Error> {
		let (tx, rx) = oneshot::channel();
		spawner.spawn(ListnersNetworkTask { local, external, result: Some(tx) })?;
		Ok(rx.await?)
	}
}
impl NetworkTask<Behaviour, Context> for ListnersNetworkTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		let mut listeners: BTreeSet<Multiaddr> = BTreeSet::new();
		if self.local {
			listeners.extend(swarm.listeners().cloned());
		}
		if self.external {
			listeners.extend(swarm.external_addresses().cloned());
		}
		if !listeners.is_empty() {
			if let Some(result) = take(&mut self.result) {
				result.send(listeners).ok();
			}
		}
	}

	fn on_swarm_event(
		&mut self,
		swarm: &mut Swarm<Behaviour>,
		context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		match &event {
			SwarmEvent::NewListenAddr { listener_id: _, address: _ } => {
				self.execute(swarm, context);
			},
			SwarmEvent::ExternalAddrConfirmed { address: _ } => {
				self.execute(swarm, context);
			},
			_ => {},
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.result.is_none()
	}
}
