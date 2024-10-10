use super::ActorHandle;
use futures::{pin_mut, Stream, StreamExt};
use std::marker::PhantomData;

/// Epic.
///
/// Defines side effects for actor actions which will produce actor messages over time.
pub trait Epic<A, S, C> {
	/// Run the epic.
	///
	/// # Arguments
	/// - `state`: The state before the action has been applied.
	fn epic(
		&mut self,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static>;
}

// pub trait BoxEpic<A, S, C> {
// 	fn box_epic(&mut self, message: &A, state: &S, context: &C) -> BoxStream<'static, Result<A, anyhow::Error>>;
// }

/// Epic runtime to be uses as actor state.
/// Expected to be called after the message has been applied to the state.
pub struct EpicRuntime<E, M, A, S, C> {
	epic: E,
	error: fn(anyhow::Error) -> Option<A>,
	// epics: Vec<Box<dyn BoxEpic<A, S, C>>>,
	_actor: PhantomData<fn(M, A, S, C)>,
}
impl<E, M, A, S, C> EpicRuntime<E, M, A, S, C>
where
	E: Epic<A, S, C>,
	A: Send + 'static + Into<M>,
	M: Send + 'static,
{
	pub fn new(epic: E, error: fn(anyhow::Error) -> Option<A>) -> Self {
		Self { _actor: Default::default(), epic, error }
	}

	pub fn handle(&mut self, actor: &ActorHandle<M>, action: &A, state: &S, context: &C) {
		let stream = self.epic.epic(action, state, context);
		if let Some(stream) = stream {
			let actor = actor.clone();
			let error = self.error;
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
pub struct JoinEpic<E1, E2> {
	a: E1,
	b: E2,
}
impl<E1, E2, A, S, C> Epic<A, S, C> for JoinEpic<E1, E2>
where
	A: Send + 'static,
	E1: Epic<A, S, C>,
	E2: Epic<A, S, C>,
{
	fn epic(
		&mut self,
		message: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + 'static> {
		let s1 = self.a.epic(message, state, context);
		let s2 = self.b.epic(message, state, context);
		let s1 = async_stream::stream! {
			if let Some(stream) = s1 {
				for await item in stream {
					yield item;
				}
			}
		};
		let s2 = async_stream::stream! {
			if let Some(stream) = s2 {
				for await item in stream {
					yield item;
				}
			}
		};
		Some(tokio_stream::StreamExt::merge(s1, s2))
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
