use super::ActorHandle;
use co_primitives::Tags;
use futures::{
	pin_mut,
	stream::{BoxStream, Empty},
	Stream, StreamExt,
};
use std::{
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
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
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
	fn join<E>(self, other: E) -> JoinEpic<Self, E>
	where
		Self: Sized,
	{
		JoinEpic(self, other)
	}

	fn once(self) -> OnceEpic<Self>
	where
		Self: Sized + Send + 'static,
	{
		OnceEpic(self, None)
	}

	fn boxed(self) -> Box<dyn BoxEpic<A, S, C> + Send + 'static>
	where
		Self: Sized + Send + 'static,
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
	epic: Box<dyn BoxEpic<A, S, C> + Send + 'static>,
	error: Arc<dyn Fn(anyhow::Error) -> Option<A> + Sync + Send + 'static>,
	// epics: Vec<Box<dyn BoxEpic<A, S, C>>>,
	_actor: PhantomData<fn(M, A, S, C)>,
}
impl<M, A, S, C> EpicRuntime<M, A, S, C>
where
	A: Send + 'static + Into<M>,
	M: Send + 'static,
{
	pub fn new(
		epic: impl EpicExt<A, S, C> + Send + 'static,
		error: impl Fn(anyhow::Error) -> Option<A> + Sync + Send + 'static,
	) -> Self {
		Self { epic: epic.boxed(), _actor: Default::default(), error: Arc::new(error) }
	}

	pub fn handle(&mut self, actor: &ActorHandle<M>, action: &A, state: &S, context: &C) {
		let stream = self.epic.box_epic(action, state, context);
		if let Some(stream) = stream {
			let actor = actor.clone();
			let error = self.error.clone();
			tokio::spawn(async move {
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
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + 'static> {
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
	use crate::Epic;
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
}

// /// Wrapps an actor with an epic into an new actor.
// pub struct EpicActor<E, P, C> {
// 	actor: P,
// 	context: C,
// 	epic: fn() -> E,
// }
// impl<E, P, C> EpicActor<E, P, C>
// where
// 	P: Actor,
// 	P::Message: Clone,
// 	E: Epic<P::Message, P::State, C> + Send + Sync + 'static,
// 	C: Send + Sync + 'static,
// {
// 	pub fn new(actor: P, epic: fn() -> E, context: C) -> Self {
// 		Self { actor, epic, context }
// 	}
// }
// pub struct EpicActorState<E, P, C>
// where
// 	P: Actor,
// 	E: Epic<P::Message, P::State, C>,
// {
// 	state: P::State,
// 	epic: EpicRuntime<E, P::Message, P::State, C>,
// }
// #[async_trait]
// impl<E, P, C> Actor for EpicActor<E, P, C>
// where
// 	P: Actor,
// 	P::Message: Clone,
// 	E: Epic<P::Message, P::State, C> + Send + Sync + 'static,
// 	C: Send + Sync + 'static,
// {
// 	type Message = P::Message;
// 	type State = EpicActorState<E, P, C>;
// 	type Initialize = P::Initialize;

// 	async fn initialize(&self, tags: Tags, initialize: Self::Initialize) -> Result<Self::State, ActorError> {
// 		let state = self.actor.initialize(tags, initialize).await?;
// 		Ok(EpicActorState { state, epic: EpicRuntime::new((self.epic)(), |_err| None) })
// 	}

// 	async fn handle(
// 		&self,
// 		handle: &ActorHandle<Self::Message>,
// 		message: Self::Message,
// 		state: &mut Self::State,
// 	) -> Result<(), ActorError> {
// 		// epic
// 		state.epic.handle(handle, &message, &state.state, &self.context);

// 		// inner
// 		self.actor.handle(handle, message, &mut state.state).await?;

// 		// result
// 		Ok(())
// 	}
// }
