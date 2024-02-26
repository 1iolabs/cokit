use super::{
	heads::{HeadsRequest, HeadsRequestNetworkTask},
	CoNetworkTaskSpawner,
};
use crate::{library::co_peer_provider::CoPeerProvider, CoReducer};
use co_network::PeerProvider;
use libp2p::PeerId;
use std::collections::BTreeSet;

pub struct Update {
	spawner: CoNetworkTaskSpawner,
	co: CoReducer,
}

impl Update {
	pub fn new(spawner: CoNetworkTaskSpawner, co: CoReducer) -> Self {
		Self { co, spawner }
	}

	pub async fn request(&self) -> Result<(), anyhow::Error> {
		let peers = CoPeerProvider::from_co_reducer(&self.co).await.peers().await?;
		self.request_peers(peers)
	}

	pub fn request_peers(&self, peers: BTreeSet<PeerId>) -> Result<(), anyhow::Error> {
		self.spawner
			.spawn(HeadsRequestNetworkTask::new(HeadsRequest::RequestHeads { co: self.co.id().clone(), peers }))?;
		Ok(())
	}
}
