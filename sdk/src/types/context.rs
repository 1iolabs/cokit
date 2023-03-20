use crate::StorageType;

#[cfg(feature = "tokio-runtime")]
pub type CoContextScheduler = rxrust::scheduler::TokioLocalScheduler;
#[cfg(feature = "futures-runtime")]
pub type CoContextScheduler = rxrust::scheduler::FuturesLocalScheduler;

pub struct CoContext {
    scheduler: CoContextScheduler,
    storage: StorageType,
}

impl CoContext {
    pub fn new(storage: StorageType, scheduler: CoContextScheduler) -> Self {
        Self { scheduler, storage }
    }

    pub fn scheduler(&self) -> CoContextScheduler {
        self.scheduler.clone()
    }

    pub fn storage(&self) -> StorageType {
        self.storage.clone()
    }
}
