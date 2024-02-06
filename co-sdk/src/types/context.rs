use crate::{ActionsType, CoStorage, StoreType};
use co_network::Libp2pNetwork;

#[cfg(feature = "tokio-runtime")]
pub type CoContextScheduler = rxrust::scheduler::TokioLocalScheduler;
#[cfg(feature = "futures-runtime")]
pub type CoContextScheduler = rxrust::scheduler::FuturesLocalScheduler;

pub struct CoContext {
	scheduler: CoContextScheduler,
	storage: CoStorage,
	store: StoreType,
	actions: ActionsType,
	network: Libp2pNetwork,
}

impl CoContext {
	pub fn new(
		network: Libp2pNetwork,
		storage: CoStorage,
		scheduler: CoContextScheduler,
		store: StoreType,
		actions: ActionsType,
	) -> Self {
		Self { network, scheduler, storage, store, actions }
	}

	/// Scheduler.
	pub fn scheduler(&self) -> CoContextScheduler {
		self.scheduler.clone()
	}

	/// Storage.
	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	/// Network.
	pub fn network(&self) -> &Libp2pNetwork {
		&self.network
	}

	/// Thread safe state store instance.
	///
	/// Warning: This is only intended to integrate other processes. Do not use directly in epics.
	pub fn store(&self) -> StoreType {
		self.store.clone()
	}

	/// Thread safe action subject.
	///
	/// Warning: This is only intended to integrate other processes. Do not use directly in epics.
	pub fn actions(&self) -> ActionsType {
		self.actions.clone()
	}
}
