use crate::types::{
	epic::{ActionObservable, BoxEpic, Epic, StateObservable},
	reducer::Reducer,
};
use rxrust::{ops::box_it::BoxOp, prelude::*};
use std::convert::Infallible;

pub struct CombineEpics<R, C>
where
	R: Reducer + 'static,
	C: Clone + 'static,
{
	epics: Vec<Box<dyn BoxEpic<R, C> + 'static>>,
}

impl<R, C> CombineEpics<R, C>
where
	R: Reducer + 'static,
	C: Clone + 'static,
{
	pub fn new() -> Self {
		Self { epics: Vec::new() }
	}

	pub fn add<E>(&mut self, epic: E) -> &mut Self
	where
		E: BoxEpic<R, C> + 'static,
	{
		self.epics.push(Box::new(epic));
		self
	}
}

impl<R, C, O> Epic<R, C, O> for CombineEpics<R, C>
where
	R: Reducer + 'static,
	C: Clone + 'static,
	O: Observer<R::Action, Infallible> + 'static,
{
	type Unsub = impl Subscription;
	type Result = impl Observable<R::Action, Infallible, O, Unsub = Self::Unsub>;

	fn epic(
		&self,
		actions: ActionObservable<R::Action>,
		states: StateObservable<R::State>,
		context: C,
	) -> Self::Result {
		let epics = self
			.epics
			.iter()
			.map(move |epic| -> BoxOp<'static, R::Action, Infallible> {
				epic.box_epic(actions.clone(), states.clone(), context.clone())
			})
			.collect::<Vec<BoxOp<'static, R::Action, Infallible>>>();
		from_iter(epics.into_iter()).flatten()
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		ActionObservable, CombineEpics, Epic, EpicMiddleware, FnReducer, StateObservable, SyncStore, SyncStoreApi,
	};
	use rxrust::{prelude::*, subscription::Subscription};
	use std::convert::Infallible;

	#[tokio::test]
	async fn example() -> anyhow::Result<()> {
		type R = FnReducer<i32, i32>;
		#[derive(Clone)]
		struct Context {}
		struct TestEpic {}
		impl<O: Observer<i32, Infallible> + 'static> Epic<R, Context, O> for TestEpic {
			type Unsub = impl Subscription;
			type Result = impl Observable<i32, Infallible, O, Unsub = Self::Unsub> + 'static;

			fn epic(
				&self,
				actions: ActionObservable<i32>,
				_states: StateObservable<i32>,
				_context: Context,
			) -> Self::Result {
				actions.filter(|i| *i > 10).map(|i| i % 10).filter(|i| *i > 0)
			}
		}
		struct TestEpic2 {}
		impl<O: Observer<i32, Infallible> + 'static> Epic<R, Context, O> for TestEpic2 {
			type Unsub = impl Subscription;
			type Result = impl Observable<i32, Infallible, O, Unsub = Self::Unsub> + 'static;

			fn epic(
				&self,
				actions: ActionObservable<i32>,
				_states: StateObservable<i32>,
				_context: Context,
			) -> Self::Result {
				actions.filter(|i| i % 10 == 0).map(|i| i / 10)
			}
		}
		// let x: Box<dyn BoxEpic<R, Context>> = Box::new(TestEpic {});
		let mut epics = CombineEpics::<R, Context>::new();
		epics.add(TestEpic {});
		epics.add(TestEpic2 {});

		let local = tokio::task::LocalSet::new();
		local
			.run_until(async move {
				let reducer = FnReducer::<i32, i32>::new(|state, action| state + action);
				let (middleware, runner, runner_subscription) = EpicMiddleware::create();
				let store = SyncStore::new(0, reducer).with_middleware(Box::new(middleware));
				let runner_handle = tokio::task::spawn_local(runner.run(Box::new(store.clone()), epics, Context {}));

				// dispatch
				store.dispatch(15).await; // 15 + 5
				store.dispatch(20).await; // 20 + 2

				// shutdown
				runner_subscription.unsubscribe();
				runner_handle.await?;

				// check
				assert_eq!(store.state().await, 42);

				// done
				Ok::<(), anyhow::Error>(())
			})
			.await?;

		// done
		Ok(())
	}
}
