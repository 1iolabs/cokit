use crate::{CoCoreResolver, CoReducer, CoStorage, Reducer, ReducerChangedHandler};
use async_trait::async_trait;
use co_network::PeerProvider;
use co_storage::StorageError;
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
		// TODO (feature_38): implmenent
		Ok(Default::default())
	}
}
