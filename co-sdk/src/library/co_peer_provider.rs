use crate::{
	state::{self, into_collection},
	CoCoreResolver, CoReducer, CoStorage, Reducer, ReducerChangedHandler,
};
use async_trait::async_trait;
use co_network::PeerProvider;
use co_primitives::Network;
use co_storage::{BlockStorageExt, StorageError};
use libipld::Cid;
use libp2p::PeerId;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct CoPeerProvider {
	storage: CoStorage,
	state: Option<Cid>,
}
impl CoPeerProvider {
	pub fn new(storage: CoStorage, state: Option<Cid>) -> Self {
		Self { storage, state }
	}

	pub async fn from_co_reducer(co: &CoReducer) -> Self {
		Self { storage: co.storage(), state: co.reducer_state().await.0 }
	}
}
#[async_trait]
impl ReducerChangedHandler<CoStorage, CoCoreResolver> for CoPeerProvider {
	async fn on_state_changed(&mut self, reducer: &Reducer<CoStorage, CoCoreResolver>) -> Result<(), anyhow::Error> {
		self.state = *reducer.state();
		Ok(())
	}
}
#[async_trait]
impl PeerProvider for CoPeerProvider {
	async fn peers(&self) -> Result<BTreeSet<PeerId>, StorageError> {
		if let Some(state) = self.state {
			let co: co_core_co::Co = self.storage.get_deserialized(&state).await?;

			// has network settings?
			if !state::is_empty(&self.storage, &co.network).await? {
				let networks: BTreeSet<Network> = into_collection(&self.storage, &co.network).await?;
			}

			// use participants
			// co.participants

			// let peers: BTreeSet<PeerId> = NodeStream::from_node_container(self.storage.clone(), &co.peers)
			// 	.map_ok(|p| PeerId::from_bytes(&p).map_err(|e| StorageError::Internal(e.into())))
			// 	.map(|p| match p {
			// 		Ok(Ok(p)) => Ok(p),
			// 		Ok(Err(e)) => Err(e),
			// 		Err(e) => Err(e),
			// 	})
			// 	.try_collect()
			// 	.await?;
			// return Ok(peers)
		}
		Ok(Default::default())
	}
}
