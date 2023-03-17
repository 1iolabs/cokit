use crate::StorageType;
use futures::Future;
use rxrust::{
    prelude::{Observable, ObservableExt, Observer},
    scheduler::{FutureTask, NormalReturn, Scheduler, TaskHandle},
};
use std::convert::Infallible;

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

    /// Future to observable.
    ///
    /// Todo: Use from rxrust when type has fixed.
    pub fn from_future<F>(&self, future: F) -> FutureObservable<F, CoContextScheduler> {
        FutureObservable {
            future,
            scheduler: self.scheduler.clone(),
        }
    }

    pub fn storage(&self) -> StorageType {
        self.storage.clone()
    }
}

pub struct FutureObservable<T, S> {
    future: T,
    scheduler: S,
}

impl<T, S, O> Observable<T::Output, Infallible, O> for FutureObservable<T, S>
where
    T: Future + 'static,
    S: Scheduler<FutureTask<T, O, NormalReturn<()>>>,
    O: Observer<T::Output, Infallible>,
{
    type Unsub = TaskHandle<NormalReturn<()>>;

    fn actual_subscribe(self, observer: O) -> Self::Unsub {
        let Self { future, scheduler } = self;
        scheduler.schedule(FutureTask::new(future, item_task, observer), None)
    }
}

impl<F: Future, S> ObservableExt<F::Output, Infallible> for FutureObservable<F, S> {}

fn item_task<Item, O>(item: Item, mut observer: O) -> NormalReturn<()>
where
    O: Observer<Item, Infallible>,
{
    observer.next(item);
    observer.complete();
    NormalReturn::new(())
}
