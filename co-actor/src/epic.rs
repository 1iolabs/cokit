use super::ActorHandle;
use crate::TaskSpawner;
use co_primitives::Tags;
use futures::{
	pin_mut,
	stream::{self, BoxStream, Empty},
	Stream, StreamExt,
};
use std::{
	any::type_name,
	fmt::Debug,
	marker::{PhantomData, Send},
	sync::Arc,
};
use tokio_util::sync::CancellationToken;

/// Epic.
///
/// Defines side effects for actions which will produce other actions over time.
pub trait Epic<A, S, C> {
	/// Run the epic.
	///
	/// # Arguments
	/// - `state`: The state after the action has been applied.
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static>;
}

/// Fn impl for epics.
impl<A, S, C, O, F> Epic<A, S, C> for F
where
	O: Stream<Item = Result<A, anyhow::Error>> + Send + 'static,
	F: FnMut(&A, &S, &C) -> Option<O>,
{
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static> {
		self(action, state, context)
	}
}

pub trait EpicExt<A, S, C>: Epic<A, S, C> {
	/// Join two Epics.
	///
	/// # Notes
	/// This will join on the stack.
	/// If you want to join dozens of epics the heap should be used.
	/// See: [`MergeEpic`].
	fn join<E>(self, other: E) -> JoinEpic<Self, E>
	where
		Self: Sized,
		A: Send + 'static,
	{
		JoinEpic(self, other)
	}

	fn once(self) -> OnceEpic<Self>
	where
		Self: Sized + Send + 'static,
	{
		OnceEpic(self, None)
	}

	fn boxed(self) -> Box<dyn BoxEpic<A, S, C> + Send + Sync + 'static>
	where
		Self: Sized + Send + Sync + 'static,
	{
		Box::new(self)
	}
}
impl<T, A, S, C> EpicExt<A, S, C> for T
where
	T: Epic<A, S, C> + ?Sized + Send + Sync + 'static,
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
{
}

/// Dynamic dispatchable epic.
pub trait BoxEpic<A, S, C> {
	fn box_epic(&mut self, action: &A, state: &S, context: &C) -> Option<BoxStream<'static, Result<A, anyhow::Error>>>;
}
impl<T, A, S, C> BoxEpic<A, S, C> for T
where
	T: Epic<A, S, C>,
{
	fn box_epic(&mut self, action: &A, state: &S, context: &C) -> Option<BoxStream<'static, Result<A, anyhow::Error>>> {
		self.epic(action, state, context).map(|stream| stream.boxed())
	}
}

/// Epic runtime to be uses as actor state.
/// Expected to be called after the message has been applied to the state.
pub struct EpicRuntime<M, A, S, C> {
	epic: Box<dyn BoxEpic<A, S, C> + Sync + Send + 'static>,
	error: Arc<dyn Fn(anyhow::Error) -> Option<A> + Sync + Send + 'static>,
	_actor: PhantomData<fn(M, A, S, C)>,
}
impl<M, A, S, C> EpicRuntime<M, A, S, C>
where
	A: Send + 'static + Into<M>,
	M: Send + 'static,
{
	pub fn new(
		epic: impl EpicExt<A, S, C> + Send + Sync + 'static,
		error: impl Fn(anyhow::Error) -> Option<A> + Sync + Send + 'static,
	) -> Self {
		Self { epic: epic.boxed(), _actor: Default::default(), error: Arc::new(error) }
	}

	pub fn handle(&mut self, spawner: &TaskSpawner, actor: &ActorHandle<M>, action: &A, state: &S, context: &C) {
		let stream = self.epic.box_epic(action, state, context);
		if let Some(stream) = stream {
			let actor = actor.clone();
			let error = self.error.clone();
			spawner.spawn_named(type_name::<A>(), async move {
				let stream = stream.take_until(actor.closed());
				pin_mut!(stream);
				while let Some(action) = stream.next().await {
					match action {
						Ok(action) => {
							actor.dispatch(action).ok();
						},
						Err(err) => {
							if let Some(action) = (error)(err) {
								actor.dispatch(action).ok();
							}
						},
					}
				}
			});
		}
	}
}
impl<M, A, S, C> Debug for EpicRuntime<M, A, S, C> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("EpicRuntime").field("_actor", &self._actor).finish()
	}
}

/// Joins two epics into one.
pub struct JoinEpic<E1, E2>(E1, E2);
impl<E1, E2, A, S, C> Epic<A, S, C> for JoinEpic<E1, E2>
where
	A: Send + 'static,
	E1: Epic<A, S, C>,
	E2: Epic<A, S, C>,
{
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static> {
		let s0 = self.0.epic(action, state, context);
		let s1 = self.1.epic(action, state, context);
		let s0 = async_stream::stream! {
			if let Some(stream) = s0 {
				for await item in stream {
					yield item;
				}
			}
		};
		let s1 = async_stream::stream! {
			if let Some(stream) = s1 {
				for await item in stream {
					yield item;
				}
			}
		};
		Some(tokio_stream::StreamExt::merge(s0, s1))
	}
}

/// Merge BoxEpic into one.
pub struct MergeEpic<A, S, C>(Vec<Box<dyn BoxEpic<A, S, C> + Send + Sync + 'static>>);
impl<A, S, C> MergeEpic<A, S, C> {
	pub fn new() -> Self {
		Self(Default::default())
	}

	pub fn join(mut self, epic: impl EpicExt<A, S, C> + Send + Sync + 'static) -> Self {
		self.0.push(epic.boxed());
		self
	}
}
impl<A, S, C> Epic<A, S, C> for MergeEpic<A, S, C>
where
	A: Send + 'static,
{
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static> {
		let streams: Vec<_> = self
			.0
			.iter_mut()
			.filter_map(|epic| epic.box_epic(&action, &state, &context))
			.collect();
		if !streams.is_empty() {
			Some(stream::iter(streams).flatten_unordered(None))
		} else {
			None
		}
	}
}

/// Trace actions and state as debug messages.
pub struct TracingEpic(Tags);
impl TracingEpic {
	pub fn new(tags: Tags) -> Self {
		Self(tags)
	}
}
impl<A, S, C> Epic<A, S, C> for TracingEpic
where
	A: Debug + Send + 'static,
	S: Debug + Send + 'static,
{
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		_context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + 'static> {
		tracing::debug!(?action, ?state, tags = ?self.0, "action");
		Option::<Empty<_>>::None
	}
}

/// Only allow to run epic once.
/// Once the epic returns another stream the previous will be dropped.
pub struct OnceEpic<E>(E, Option<CancellationToken>);
impl<E, A, S, C> Epic<A, S, C> for OnceEpic<E>
where
	E: Epic<A, S, C>,
	A: Debug + Send + 'static,
	S: Debug + Send + 'static,
{
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + 'static> {
		let next = self.0.epic(action, state, context);
		match next {
			Some(stream) => {
				// cancel previous
				if let Some(cancel) = self.1.take() {
					cancel.cancel();
				}

				// create next
				let token = CancellationToken::new();
				self.1 = Some(token.clone());
				Some(stream.take_until(token.cancelled_owned()))
			},
			None => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{Epic, EpicExt};
	use futures::{stream, Stream, TryStreamExt};

	#[derive(Debug, Clone, PartialEq)]
	enum TestAction {
		Hello,
		World,
	}
	struct Test {}
	impl Epic<TestAction, (), ()> for Test {
		fn epic(
			&mut self,
			action: &TestAction,
			_state: &(),
			_context: &(),
		) -> Option<impl Stream<Item = Result<TestAction, anyhow::Error>> + Send + 'static> {
			match action {
				TestAction::Hello => Some(stream::once(async { Ok(TestAction::World) })),
				_ => None,
			}
		}
	}

	#[tokio::test]
	async fn test_hello() {
		let mut epic = Test {};
		let result: Vec<TestAction> = epic
			.epic(&TestAction::Hello, &(), &())
			.expect("a stream")
			.try_collect()
			.await
			.expect("no error");
		assert_eq!(result, vec![TestAction::World]);
	}

	#[tokio::test]
	async fn test_fn_epic() {
		fn test(
			action: &TestAction,
			_state: &(),
			_context: &(),
		) -> Option<impl Stream<Item = Result<TestAction, anyhow::Error>> + Send + 'static> {
			match action {
				TestAction::Hello => Some(stream::once(async { Ok(TestAction::World) })),
				_ => None,
			}
		}
		let result: Vec<TestAction> = test
			.epic(&TestAction::Hello, &(), &())
			.expect("a stream")
			.try_collect()
			.await
			.expect("no error");
		assert_eq!(result, vec![TestAction::World]);
	}

	#[tokio::test]
	async fn test_box_epic() {
		let mut epic = Test {}.boxed();
		let result: Vec<TestAction> = epic
			.box_epic(&TestAction::Hello, &(), &())
			.expect("a stream")
			.try_collect()
			.await
			.expect("no error");
		assert_eq!(result, vec![TestAction::World]);
	}
}
