use std::convert::Infallible;

use futures::{executor::{LocalPool, LocalSpawner}, Future};
use rxrust::{prelude::{Observer, Observable, ObservableExt}, scheduler::{TaskHandle, NormalReturn, Scheduler, FutureTask}};

pub struct CoContext
{
    pool: LocalPool, // todo: replace with LocalSet?
}

impl CoContext
{
    pub fn new() -> Self {
        Self {
            pool: LocalPool::new(),
        }
    }

    /// Future to observable.
    /// 
    /// Todo: Use from rxrust when type has fixed.
    pub fn from_future<F>(&self, future: F) -> FutureObservable<F, LocalSpawner> {
        FutureObservable {
            future,
            scheduler: self.pool.spawner(),
        }
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
