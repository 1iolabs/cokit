use crate::{ActionsType, StorageType, StoreType};

#[cfg(feature = "tokio-runtime")]
pub type CoContextScheduler = rxrust::scheduler::TokioLocalScheduler;
#[cfg(feature = "futures-runtime")]
pub type CoContextScheduler = rxrust::scheduler::FuturesLocalScheduler;

pub struct CoContext {
    scheduler: CoContextScheduler,
    storage: StorageType,
    store: StoreType,
    actions: ActionsType,
}

impl CoContext {
    pub fn new(
        storage: StorageType,
        scheduler: CoContextScheduler,
        store: StoreType,
        actions: ActionsType,
    ) -> Self {
        Self {
            scheduler,
            storage,
            store,
            actions,
        }
    }

    pub fn scheduler(&self) -> CoContextScheduler {
        self.scheduler.clone()
    }

    pub fn storage(&self) -> StorageType {
        self.storage.clone()
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
