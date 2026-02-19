// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Action, Middleware, Reducer, State, Store, StoreApi, SyncStoreApi};
use rxrust::{
	prelude::{from_stream, Observable, ObservableExt, Observer},
	scheduler::{NormalReturn, TaskHandle},
};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::watch;
use std::sync::Mutex;

pub struct SyncStore<R>
where
	R: Reducer + Send + 'static,
{
	store: Arc<Mutex<Box<dyn StoreApi<R> + Send + 'static>>>,
	subscriptions: watch::Receiver<R::State>,
}

impl<R> Clone for SyncStore<R>
where
	R: Reducer + Send + 'static,
{
	fn clone(&self) -> Self {
		Self { store: self.store.clone(), subscriptions: self.subscriptions.clone() }
	}
}

impl<R> SyncStore<R>
where
	R: Reducer + Send + 'static,
	R::Action: Action,
	R::State: State,
{
	pub fn new(state: R::State, reducer: R) -> Self {
		let (tx, rx) = watch::channel(state.clone());
		let on_changed = move |state: &R::State| tx.send(state.clone()).expect("Send state subscription failed");
		Self {
			store: Arc::new(Mutex::new(Box::new(Store::new(state.clone(), on_changed, reducer)))),
			subscriptions: rx,
		}
	}

	pub fn with_middleware(self, middleware: Box<dyn Middleware<R> + Send + 'static>) -> Self {
		let store = Arc::try_unwrap(self.store)
			.expect("Can only call with_middleware() when store has not yet been cloned.")
			.into_inner()
			.with_middleware(middleware);
		Self { store: Arc::new(Mutex::new(store)), subscriptions: self.subscriptions }
	}
}

#[async_trait::async_trait]
impl<R> SyncStoreApi<R> for SyncStore<R>
where
	R: Reducer + Send + 'static,
{
	async fn dispatch(&self, action: R::Action) {
		self.store.lock().unwrap().dispatch(action);
	}

	async fn state(&self) -> R::State {
		self.store.lock().unwrap().state()
	}
}

// impl<R> Observable for SyncStore<R>
// where
//     R: Reducer + Send + 'static,
//     R::State: State,
//     R::Action: Action,
// {
//     //type Item = R::State;
//     //type Subscription = impl Stream<Item = Self::Item> + Sync + 'static;

//     /// Subscribe to state change watcher.
//     /// Note: States may skipped if happen simultanious. Only the last result will be handled.
//     fn subscribe(&self) -> Self::Subscription {
//         let mut rx = self.subscriptions.clone();
//         async_stream::stream! {
//             while rx.changed().await.is_ok() {
//                 let state = rx.borrow().clone();
//                 yield state;
//             }
//         }
//     }
// }

impl<R, O> Observable<R::State, Infallible, O> for SyncStore<R>
where
	R: Reducer + Send + 'static,
	O: Observer<R::State, Infallible> + Send + 'static,
{
	type Unsub = TaskHandle<NormalReturn<()>>;

	fn actual_subscribe(self, observer: O) -> Self::Unsub {
		let mut rx = self.subscriptions;
		let stream = async_stream::stream! {
			while rx.changed().await.is_ok() {
				let state = rx.borrow().clone();
				yield state;
			}
		};
		from_stream(stream, tokio::runtime::Handle::current()).actual_subscribe(observer)
	}
}
impl<R> ObservableExt<R::State, Infallible> for SyncStore<R> where R: Reducer + Send + 'static {}

#[cfg(test)]
mod tests {
	use crate::{FnReducer, Reducer, SyncStore, SyncStoreApi};
	use rxrust::prelude::*;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestState {
		count: i32,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	enum TestAction {
		IncCount,
		DecCount,
	}

	impl Reducer for TestState {
		type State = TestState;
		type Action = TestAction;
		fn reduce(&self, state: TestState, action: &TestAction) -> TestState {
			match action {
				TestAction::IncCount => TestState { count: state.count + 1, ..state },
				TestAction::DecCount => TestState { count: state.count - 1, ..state },
			}
		}
	}

	#[test]
	fn send() {
		static_assertions::assert_impl_all!(SyncStore<TestState>: Send);
		static_assertions::assert_impl_all!(SyncStore<TestState>: Sync);
	}

	#[tokio::test]
	async fn full() -> anyhow::Result<()> {
		// start
		let reducer = FnReducer::new(|state: TestState, action: &TestAction| match action {
			TestAction::IncCount => TestState { count: state.count + 1, ..state },
			TestAction::DecCount => TestState { count: state.count - 1, ..state },
		});
		let store = SyncStore::new(TestState { count: 0 }, reducer);

		// subscribe
		let subscription_handle = tokio::task::spawn(
			store.clone().collect::<Vec<_>>().to_future(), /* rxrust::prelude::ObservableExt::collect(store.clone())
			                                                * .subscribe()
			                                                * .map(|v| {println!("state: {:?}", &v); v})
			                                                * .collect::<Vec<_>>(), */
		);
		// tokio::time::sleep(std::time::Duration::from_millis(100)).await;

		// dispatch
		store.dispatch(TestAction::IncCount).await;
		// tokio::time::sleep(std::time::Duration::from_millis(100)).await;
		store.dispatch(TestAction::IncCount).await;
		// tokio::time::sleep(std::time::Duration::from_millis(100)).await;
		store.dispatch(TestAction::DecCount).await;
		// tokio::time::sleep(std::time::Duration::from_millis(100)).await;

		// drop store which will also end the subscriptions
		drop(store);

		// test
		let subscriptions = subscription_handle.await???;
		assert!(subscriptions.len() > 0);
		assert_eq!(subscriptions.last().unwrap().count, 1);

		// done
		Ok(())
	}
}
