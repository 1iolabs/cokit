// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Middleware, Reducer, StoreApi, SyncStoreApi};
#[cfg(feature = "futures-runtime")]
use futures::channel::mpsc::{unbounded as unbounded_channel, UnboundedReceiver, UnboundedSender};
use rxrust::{observer::BoxObserver, ops::box_it::BoxOp, prelude::*};
use std::{convert::Infallible, fmt::Debug};
#[cfg(feature = "tokio-runtime")]
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub trait Epic<R, C, O>
where
	R: Reducer + 'static,
	C: Clone + 'static,
	O: Observer<R::Action, Infallible> + 'static,
{
	type Unsub: Subscription;
	type Result: Observable<R::Action, Infallible, O, Unsub = Self::Unsub> + 'static;

	fn epic(&self, actions: ActionObservable<R::Action>, states: StateObservable<R::State>, context: C)
		-> Self::Result;
}

/// Epic implementation for function epics.
impl<R, C, O, T, F> Epic<R, C, O> for F
where
	R: Reducer + 'static,
	C: Clone + 'static,
	F: Fn(ActionObservable<R::Action>, StateObservable<R::State>, C) -> T,
	O: Observer<R::Action, Infallible> + 'static,
	T: Observable<R::Action, Infallible, O> + 'static,
{
	type Result = T;
	type Unsub = T::Unsub;

	fn epic(
		&self,
		actions: ActionObservable<R::Action>,
		states: StateObservable<R::State>,
		context: C,
	) -> Self::Result {
		self(actions, states, context)
	}
}

pub trait BoxEpic<R, C>
where
	R: Reducer + 'static,
	C: Clone + 'static,
{
	fn box_epic(
		&self,
		actions: ActionObservable<R::Action>,
		states: StateObservable<R::State>,
		context: C,
	) -> BoxOp<'static, R::Action, Infallible>;
}
impl<R, C, E> BoxEpic<R, C> for E
where
	R: Reducer + 'static,
	C: Clone + 'static,
	E: Epic<R, C, BoxObserver<'static, R::Action, Infallible>>,
	E::Result: Observable<R::Action, Infallible, BoxObserver<'static, R::Action, Infallible>, Unsub = E::Unsub>
		+ BoxIt<BoxOp<'static, R::Action, Infallible>>,
	// <<E as Epic<R, C, BoxObserverThreads<<R as Reducer>::Action, Infallible>>>::Result as Observable::<R::Action,
	// Infallible, BoxObserverThreads<R::Action, Infallible>>>::Unsub: Send + 'static,
{
	fn box_epic(
		&self,
		actions: ActionObservable<R::Action>,
		states: StateObservable<R::State>,
		context: C,
	) -> BoxOp<'static, R::Action, Infallible> {
		self.epic(actions, states, context).box_it()
	}
}

#[derive(Clone)]
pub struct EpicObserver<R>
where
	R: Reducer + 'static,
{
	commands: UnboundedSender<EpicCommand<R>>,
}

impl<R> Observer<R::Action, Infallible> for EpicObserver<R>
where
	R: Reducer + 'static,
{
	fn next(&mut self, value: R::Action) {
		self.commands.send(EpicCommand::Dispatch(value)).unwrap();
	}

	fn error(self, _err: Infallible) {
		// self.commands.send(EpicCommand::Error(err)).unwrap();
	}

	fn complete(self) {
		self.commands.send(EpicCommand::Complete).unwrap();
	}

	fn is_finished(&self) -> bool {
		false
	}
}

#[derive(Clone)]
pub struct ActionObservable<T> {
	subject: Subject<'static, T, Infallible>,
}
impl<T> ActionObservable<T> {
	pub fn new() -> Self {
		Self { subject: Subject::default() }
	}
}
impl<T, O> Observable<T, Infallible, O> for ActionObservable<T>
where
	O: Observer<T, Infallible> + 'static,
{
	type Unsub = Subscriber<O>;

	fn actual_subscribe(self, observer: O) -> Self::Unsub {
		self.subject.actual_subscribe(observer)
	}
}
impl<T> ObservableExt<T, Infallible> for ActionObservable<T> {}

#[derive(Clone)]
pub struct StateObservable<T> {
	subject: Subject<'static, T, Infallible>,
}
impl<T> StateObservable<T> {
	pub fn new() -> Self {
		Self { subject: Subject::default() }
	}
}
impl<T, O> Observable<T, Infallible, O> for StateObservable<T>
where
	O: Observer<T, Infallible> + 'static,
{
	type Unsub = Subscriber<O>;

	fn actual_subscribe(self, observer: O) -> Self::Unsub {
		self.subject.actual_subscribe(observer)
	}
}
impl<T> ObservableExt<T, Infallible> for StateObservable<T> {}

/// Epic Middleware. Uses reducer wrapping.
pub struct EpicMiddleware<R>
where
	R: Reducer + 'static,
{
	commands_tx: UnboundedSender<EpicCommand<R>>,
}

impl<R> EpicMiddleware<R>
where
	R: Reducer + 'static,
{
	pub fn create() -> (Self, EpicRunner<R>, EpicSubscription<R>) {
		let (commands_tx, commands_rx) = unbounded_channel();

		let runner = EpicRunner { commands_tx: commands_tx.clone(), commands_rx };
		let subscription = EpicSubscription { commands_tx: commands_tx.clone() };
		(Self { commands_tx }, runner, subscription)
	}
}

pub struct EpicSubscription<R>
where
	R: Reducer + 'static,
{
	commands_tx: UnboundedSender<EpicCommand<R>>,
}
impl<R> Subscription for EpicSubscription<R>
where
	R: Reducer + 'static,
{
	fn unsubscribe(self) {
		self.commands_tx.send(EpicCommand::Shutdown).unwrap();
	}

	fn is_closed(&self) -> bool {
		self.commands_tx.is_closed()
	}
}

pub struct EpicRunner<R>
where
	R: Reducer + 'static,
{
	commands_tx: UnboundedSender<EpicCommand<R>>,
	commands_rx: UnboundedReceiver<EpicCommand<R>>,
}
impl<R> EpicRunner<R>
where
	R: Reducer + Send + 'static,
{
	pub async fn run<E, C>(mut self, store: Box<dyn SyncStoreApi<R> + Send + Sync + 'static>, epic: E, context: C)
	where
		C: Clone + 'static,
		E: Epic<R, C, EpicObserver<R>> + 'static,
	{
		// log
		tracing::debug!("epic-runner-starting");

		// execute epic
		let mut actions = Some(ActionObservable::<R::Action>::new());
		let mut states = Some(StateObservable::<R::State>::new());
		let result_observable = epic.epic(actions.clone().unwrap(), states.clone().unwrap(), context);
		result_observable.actual_subscribe(EpicObserver { commands: self.commands_tx.clone() });

		// execute commands
		while let Some(command) = self.commands_rx.recv().await {
			match command {
				EpicCommand::Dispatch(action) => {
					store.dispatch(action).await;
				},
				EpicCommand::State(state, action) => {
					if let Some(s) = states.as_mut() {
						s.subject.next(state);
					}
					if let Some(actions) = actions.as_mut() {
						actions.subject.next(action);
					}
				},
				EpicCommand::Shutdown => {
					if states.is_some() {
						states.take().unwrap().subject.complete();
					}
					if actions.is_some() {
						actions.take().unwrap().subject.complete();
					}
					// todo: add timeout?
				},
				EpicCommand::Complete => break, /* EpicCommand::Error(err) => {
				                                 *     return Err(err);
				                                 * } */
			}
		}

		// log
		tracing::debug!("epic-runner-stopped");
	}
}

enum EpicCommand<R>
where
	R: Reducer + 'static,
{
	/// State changed in reducer.
	State(R::State, R::Action),
	/// Result action from epic.
	Dispatch(R::Action),
	/// Graceful shutdown.
	Shutdown,
	/// Epic has completed.
	Complete,
	// /// Epic has failed.
	// Error(anyhow::Error),
}

impl<R> Debug for EpicCommand<R>
where
	R: Reducer + 'static,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::State(arg0, arg1) => f.debug_tuple("State").field(arg0).field(arg1).finish(),
			Self::Dispatch(arg0) => f.debug_tuple("Dispatch").field(arg0).finish(),
			Self::Shutdown => write!(f, "Shutdown"),
			Self::Complete => write!(f, "Complete"),
			// Self::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
		}
	}
}

impl<R> Middleware<R> for EpicMiddleware<R>
where
	R: Reducer + 'static,
{
	fn dispatch<'a>(&mut self, next: &'a mut dyn StoreApi<R>, action: R::Action) {
		// next
		next.dispatch(action.clone());

		// epic
		self.commands_tx.send(EpicCommand::State(next.state(), action)).unwrap();
	}
}

#[cfg(test)]
mod tests {

	#[cfg(feature = "tokio-runtime")]
	mod tokio_runtime {
		use crate::{ActionObservable, Epic, EpicMiddleware, FnReducer, StateObservable, SyncStore, SyncStoreApi};
		use rxrust::{
			prelude::{Observable, ObservableExt, Observer},
			subscription::Subscription,
		};
		use std::convert::Infallible;

		#[tokio::test]
		async fn function_epic() -> anyhow::Result<()> {
			fn my_epic<O: Observer<i32, Infallible> + 'static>(
				actions: ActionObservable<i32>,
				_states: StateObservable<i32>,
				_context: (),
			) -> impl Observable<i32, Infallible, O> {
				actions.filter(|i| *i > 10).map(|i| i % 10)
			}

			let local = tokio::task::LocalSet::new();
			local
				.run_until(async move {
					let reducer = FnReducer::<i32, i32>::new(|state, action| state + action);
					let (middleware, runner, runner_subscription) = EpicMiddleware::create();
					let store = SyncStore::new(0, reducer).with_middleware(Box::new(middleware));
					let runner_handle = tokio::task::spawn_local(runner.run(Box::new(store.clone()), my_epic, ()));

					// dispatch
					store.dispatch(15).await;

					// shutdown
					runner_subscription.unsubscribe();
					runner_handle.await?;

					// check
					assert_eq!(store.state().await, 20);

					// done
					Ok::<(), anyhow::Error>(())
				})
				.await?;
			Ok(())
		}

		#[tokio::test]
		async fn example() -> anyhow::Result<()> {
			struct TestEpic {}
			impl<O: Observer<i32, Infallible> + 'static> Epic<FnReducer<i32, i32>, (), O> for TestEpic {
				type Unsub = impl Subscription;
				type Result = impl Observable<i32, Infallible, O, Unsub = Self::Unsub>;

				fn epic(
					&self,
					actions: ActionObservable<i32>,
					_states: StateObservable<i32>,
					_context: (),
				) -> Self::Result {
					actions.filter(|i| *i > 10).map(|i| i % 10)
				}
			}

			// fn my_epic(actions: ActionObservable<i32>, states: StateObservable<i32>, context: Context) -> impl
			// Observable<i32, Infallible, EpicObserver<FnReducer<i32, i32>>> {    actions.filter(|i| *i > 10).map(|i| i
			// % 10) }

			let local = tokio::task::LocalSet::new();
			local
				.run_until(async move {
					let reducer = FnReducer::<i32, i32>::new(|state, action| state + action);
					let (middleware, runner, runner_subscription) = EpicMiddleware::create();
					let store = SyncStore::new(0, reducer).with_middleware(Box::new(middleware));
					let runner_handle = tokio::task::spawn_local(runner.run(Box::new(store.clone()), TestEpic {}, ()));

					// dispatch
					store.dispatch(15).await;

					// shutdown
					runner_subscription.unsubscribe();
					runner_handle.await?;

					// check
					assert_eq!(store.state().await, 20);

					// done
					Ok::<(), anyhow::Error>(())
				})
				.await?;

			// done
			Ok(())
		}
	}

	#[cfg(feature = "futures-runtime")]
	mod futures_runtime {
		use crate::{ActionObservable, Epic, EpicMiddleware, FnReducer, StateObservable, SyncStore, SyncStoreApi};
		use rxrust::{
			prelude::{Observable, ObservableExt, Observer},
			subscription::Subscription,
		};
		use std::convert::Infallible;

		#[test]
		async fn futures_executor() {
			use futures::{executor::LocalPool, task::LocalSpawnExt};

			fn my_epic<O: Observer<i32, Infallible> + 'static>(
				actions: ActionObservable<i32>,
				_states: StateObservable<i32>,
				_context: (),
			) -> impl Observable<i32, Infallible, O> {
				actions.filter(|i| *i > 10).map(|i| i % 10)
			}

			let reducer = FnReducer::<i32, i32>::new(|state, action| state + action);
			let (middleware, runner, runner_subscription) = EpicMiddleware::create();
			let store = SyncStore::new(0, reducer).with_middleware(Box::new(middleware));

			// setup
			let mut local = LocalPool::new();
			let spawner = local.spawner();
			spawner.spawn_local(runner.run(Box::new(store.clone()), my_epic, ())).unwrap();
			spawner
				.spawn_local(async move {
					// dispatch
					store.dispatch(15).await;

					// shutdown
					runner_subscription.unsubscribe();

					// check
					assert_eq!(store.state().await, 20);
				})
				.unwrap();

			// run
			local.run();
		}
	}
}
