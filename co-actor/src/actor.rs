use crate::{Response, ResponseReceiver, ResponseStream, ResponseStreamReceiver, TaskSpawner};
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::Tags;
use futures::Stream;
use tokio::{
	sync::{mpsc, watch},
	task::JoinHandle,
};
use tracing::Instrument;

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
	#[error("Invalid actor state for that operation.")]
	InvalidState(#[source] anyhow::Error),

	#[error("Operation canceled.")]
	Canceled,

	#[error("Actor error")]
	Actor(#[source] anyhow::Error),
}

/// Simple actor model implemetation.
/// Accepts messages which will be applied to the actor state.
/// Actor state is different to the actual actor instance in order to allow initialization of it within the actor
/// context.
#[async_trait]
pub trait Actor: Send + Sync + 'static {
	type Message: Send + 'static;
	type State: Send + 'static;
	type Initialize: Send + 'static;

	async fn initialize(
		&self,
		handle: &ActorHandle<Self::Message>,
		tags: Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError>;

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError>;

	fn tags(&self, tags: Tags) -> Result<Tags, ActorError> {
		Ok(tags)
	}

	/// Spawn actor.
	fn spawn(tags: Tags, actor: Self, initialize: Self::Initialize) -> Result<ActorInstance<Self>, ActorError>
	where
		Self: Send + Sized + 'static,
	{
		Self::spawn_with(Default::default(), tags, actor, initialize)
	}

	/// Spawn actor using a task spawner.
	/// TODO: to simplyfy lifecycle make async and wait for intitalize?
	fn spawn_with(
		spawner: TaskSpawner,
		tags: Tags,
		actor: Self,
		initialize: Self::Initialize,
	) -> Result<ActorInstance<Self>, ActorError>
	where
		Self: Send + Sized + 'static,
	{
		let span = tracing::trace_span!("actor", ?tags);
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		let (state_tx, state_rx) = watch::channel(ActorState::Starting);
		let tags = actor.tags(tags)?;
		let handle = ActorHandle { tx: tx.clone(), state: state_rx.clone() };
		let join = spawner.spawn({
			let tags = tags.clone();
			let handle = handle.clone();
			async move {
				// initialize
				let mut actor_state = actor.initialize(&handle, tags, initialize).await?;
				state_tx
					.send(ActorState::Running)
					.map_err(|e| ActorError::InvalidState(e.into()))?;

				// execute
				while let Some(actor_message) = rx.recv().await {
					match actor_message {
						ActorMessage::Message(message) => {
							actor.handle(&handle, message, &mut actor_state).await?;
						},
						ActorMessage::Shutdown => {
							state_tx
								.send(ActorState::Stopping)
								.map_err(|e| ActorError::InvalidState(e.into()))?;
							rx.close();
							break;
						},
					}
				}

				// done
				state_tx
					.send(ActorState::None)
					.map_err(|e| ActorError::InvalidState(e.into()))?;
				Ok(())
			}
			.instrument(span)
		});
		Ok(ActorInstance { join, handle, tags })
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
#[repr(u8)]
pub enum ActorState {
	/// Starting.
	Starting,

	/// Running.
	Running,

	/// Shutdown has been requested.
	Stopping,

	/// Not running (yet or anymore).
	None,
}

#[derive(Debug)]
enum ActorMessage<M> {
	/// Actor shutdown requested.
	Shutdown,

	/// Actor received message.
	Message(M),
}

/// The actual actor instance.
pub struct ActorInstance<A>
where
	A: Actor,
{
	handle: ActorHandle<A::Message>,
	join: JoinHandle<Result<(), ActorError>>,
	tags: Tags,
}
impl<A> ActorInstance<A>
where
	A: Actor,
{
	/// Get actor handle.
	pub fn handle(&self) -> ActorHandle<A::Message> {
		self.handle.clone()
	}

	/// Get actor tags.
	pub fn tags(&self) -> Tags {
		self.tags.clone()
	}

	/// Request shutdown.
	pub fn shutdown(&self) {
		self.handle().shutdown();
	}

	/// Wait until the actor completes.
	pub async fn join(self) -> Result<(), ActorError> {
		self.join.await.map_err(|e| ActorError::InvalidState(e.into()))??;
		Ok(())
	}

	/// Get actor state.
	pub fn state(&self) -> ActorState {
		*self.handle.state.borrow()
	}
}

/// Handle into an actor which can be used to send messages.
#[derive(Debug)]
pub struct ActorHandle<M> {
	tx: mpsc::UnboundedSender<ActorMessage<M>>,
	state: watch::Receiver<ActorState>,
}
impl<M> Clone for ActorHandle<M> {
	fn clone(&self) -> Self {
		Self { tx: self.tx.clone(), state: self.state.clone() }
	}
}
impl<M> ActorHandle<M>
where
	M: Send + 'static,
{
	/// Wait for startup to be complete.
	pub async fn initialized(&self) -> Result<(), ActorError> {
		let mut state = self.state.clone();
		loop {
			let actor_state = *state.borrow_and_update();
			match actor_state {
				ActorState::Starting => {
					state.changed().await.map_err(|e| ActorError::InvalidState(e.into()))?;
				},
				_ => {
					break;
				},
			}
		}
		Ok(())
	}

	/// Wait for actor shutdown.
	pub async fn closed(&self) -> Result<(), ActorError> {
		let mut state = self.state.clone();
		loop {
			let actor_state = *state.borrow_and_update();
			match actor_state {
				ActorState::Starting | ActorState::Running => {
					state.changed().await.map_err(|e| ActorError::InvalidState(e.into()))?;
				},
				_ => {
					break;
				},
			}
		}
		Ok(())
	}

	/// Request shutdown.
	pub fn shutdown(&self) {
		self.tx.send(ActorMessage::Shutdown).ok();
	}

	/// Dispatch message.
	/// Will only fail when the actor already has been stopped.
	pub fn dispatch(&self, message: impl Into<M>) -> Result<(), ActorError> {
		self.tx
			.send(ActorMessage::Message(message.into()))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running.")))?;
		Ok(())
	}

	/// Request with response.
	pub async fn request<T>(&self, message: impl FnOnce(Response<T>) -> M) -> Result<T, ActorError> {
		let (responder, response) = ResponseReceiver::new();
		self.tx
			.send(ActorMessage::Message(message(responder)))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running.")))?;
		response.await
	}

	/// Request with streaming response.
	pub fn stream<T>(&self, message: impl FnOnce(ResponseStream<T>) -> M) -> impl Stream<Item = Result<T, ActorError>> {
		let (responder, response) = ResponseStreamReceiver::new();
		let send_result = self
			.tx
			.send(ActorMessage::Message(message(responder)))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running.")));
		async_stream::stream! {
			// fail if send not worked
			match send_result {
				Ok(_) => {},
				Err(err) => {
					yield Err(err);
					return;
				}
			}

			// forward items
			for await item in response {
				match item {
					Err(ActorError::Canceled) => {
						break;
					},
					item => {
						yield item;
					},
				}
			}
		}
	}
}

// pub trait ActorExt: Actor {
// 	fn with_epic<E, C>(self, epic: E, context: C) -> EpicActor<Self, C>
// 	where
// 		E: Epic<Self::Message, Self::State, C>,
// 	{
// 		EpicActor { actor: self, context }
// 	}
// }

#[cfg(test)]
mod tests {
	use crate::{Actor, ActorError, ActorHandle, Response, ResponseStream, ResponseStreams};
	use async_trait::async_trait;
	use co_primitives::Tags;
	use futures::{StreamExt, TryStreamExt};

	#[tokio::test]
	async fn smoke() {
		struct Test {}
		enum TestMessage {
			Inc(i32),
			Get(Response<i32>),
			IncGet(i32, Response<i32>),
		}

		#[async_trait]
		impl Actor for Test {
			type Message = TestMessage;
			type State = i32;
			type Initialize = i32;

			async fn initialize(
				&self,
				_handle: &ActorHandle<Self::Message>,
				_tags: Tags,
				initialize: Self::Initialize,
			) -> Result<Self::State, ActorError> {
				Ok(initialize)
			}

			async fn handle(
				&self,
				_handle: &ActorHandle<Self::Message>,
				message: Self::Message,
				state: &mut Self::State,
			) -> Result<(), ActorError> {
				match message {
					TestMessage::Inc(value) => {
						*state = value + *state;
					},
					TestMessage::Get(response) => {
						response.respond(*state).ok();
					},
					TestMessage::IncGet(value, response) => {
						*state = value + *state;
						response.respond(*state).ok();
					},
				}
				Ok(())
			}
		}

		let actor = Actor::spawn(Default::default(), Test {}, 0).unwrap();
		let handle = actor.handle();
		handle.dispatch(TestMessage::Inc(10)).unwrap();
		handle.dispatch(TestMessage::Inc(-5)).unwrap();
		let state = handle.request(TestMessage::Get).await.unwrap();
		assert_eq!(state, 5);
		let state = handle.request(|r| TestMessage::IncGet(37, r)).await.unwrap();
		assert_eq!(state, 42);
	}

	#[tokio::test]
	async fn test_stream() {
		struct Test {}
		enum TestMessage {
			Inc(i32),
			Watch(ResponseStream<i32>),
		}
		struct TestState {
			watchers: ResponseStreams<i32>,
			value: i32,
		}

		#[async_trait]
		impl Actor for Test {
			type Message = TestMessage;
			type State = TestState;
			type Initialize = i32;

			async fn initialize(
				&self,
				_handle: &ActorHandle<Self::Message>,
				_tags: Tags,
				initialize: Self::Initialize,
			) -> Result<Self::State, ActorError> {
				Ok(TestState { watchers: Default::default(), value: initialize })
			}

			async fn handle(
				&self,
				_handle: &ActorHandle<Self::Message>,
				message: Self::Message,
				state: &mut Self::State,
			) -> Result<(), ActorError> {
				match message {
					TestMessage::Inc(value) => {
						state.value = value + state.value;
						state.watchers.send(state.value);
					},
					TestMessage::Watch(mut response) => {
						if response.send(state.value).is_ok() {
							state.watchers.push(response);
						}
					},
				}
				Ok(())
			}
		}

		let actor = Actor::spawn(Default::default(), Test {}, 0).unwrap();
		let handle = actor.handle();
		handle.dispatch(TestMessage::Inc(10)).unwrap();
		handle.dispatch(TestMessage::Inc(-1)).unwrap();
		let state = handle.stream(TestMessage::Watch);
		handle.dispatch(TestMessage::Inc(-4)).unwrap();
		handle.dispatch(TestMessage::Inc(37)).unwrap();
		let result: Vec<i32> = state.take(3).try_collect().await.unwrap();
		assert_eq!(result, vec![9, 5, 42]);
	}
}
