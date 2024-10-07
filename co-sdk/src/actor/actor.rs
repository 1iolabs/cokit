use super::{Response, ResponseReceiver, ResponseStream, ResponseStreamReceiver};
use crate::Tags;
use anyhow::anyhow;
use async_trait::async_trait;
use futures::Stream;
use tokio::{
	sync::{mpsc, watch},
	task::JoinHandle,
};

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

	async fn initialize(&mut self, tags: Tags, initialize: Self::Initialize) -> Result<Self::State, ActorError>;
	async fn handle(&self, message: Self::Message, state: &mut Self::State) -> Result<(), ActorError>;

	fn tags(&self, tags: Tags) -> Result<Tags, ActorError> {
		Ok(tags)
	}

	/// TODO: to simplyfy lifecycle make async and wait for intitalize?
	fn spawn(tags: Tags, actor: Self, initialize: Self::Initialize) -> Result<ActorInstance<Self>, ActorError>
	where
		Self: Send + Sized + 'static,
	{
		let mut actor = actor;
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
		let (state_tx, state_rx) = watch::channel(ActorState::Starting);
		let tags = actor.tags(tags)?;
		let join = tokio::spawn({
			let tags = tags.clone();
			async move {
				// initialize
				let mut actor_state = actor.initialize(tags, initialize).await?;
				state_tx
					.send(ActorState::Running)
					.map_err(|e| ActorError::InvalidState(e.into()))?;

				// execute
				while let Some(actor_message) = rx.recv().await {
					match actor_message {
						ActorMessage::Message(message) => {
							actor.handle(message, &mut actor_state).await?;
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
		});
		Ok(ActorInstance { join, tx, state: state_rx, tags })
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
	tx: mpsc::UnboundedSender<ActorMessage<A::Message>>,
	join: JoinHandle<Result<(), ActorError>>,
	state: watch::Receiver<ActorState>,
	tags: Tags,
}
impl<A> ActorInstance<A>
where
	A: Actor,
{
	/// Get actor handle.
	pub fn handle(&self) -> ActorHandle<A::Message> {
		ActorHandle { tx: self.tx.clone() }
	}

	/// Get actor tags.
	pub fn tags(&self) -> Tags {
		self.tags.clone()
	}

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
		*self.state.borrow()
	}
}

/// Handle into an actor which can be used to send messages.
pub struct ActorHandle<M> {
	tx: mpsc::UnboundedSender<ActorMessage<M>>,
}
impl<M> ActorHandle<M>
where
	M: Send + 'static,
{
	/// Request shutdown.
	pub fn shutdown(&self) {
		self.tx.send(ActorMessage::Shutdown).ok();
	}

	/// Dispatch message.
	/// Will only fail when the actor already has been stopped.
	pub fn dispatch(&self, message: M) -> Result<(), ActorError> {
		self.tx
			.send(ActorMessage::Message(message))
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

#[cfg(test)]
mod tests {
	use crate::actor::{Actor, ActorError, Response, ResponseStream, ResponseStreams};
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
				&mut self,
				_tags: Tags,
				initialize: Self::Initialize,
			) -> Result<Self::State, ActorError> {
				Ok(initialize)
			}

			async fn handle(&self, message: Self::Message, state: &mut Self::State) -> Result<(), ActorError> {
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
				&mut self,
				_tags: Tags,
				initialize: Self::Initialize,
			) -> Result<Self::State, ActorError> {
				Ok(TestState { watchers: Default::default(), value: initialize })
			}

			async fn handle(&self, message: Self::Message, state: &mut Self::State) -> Result<(), ActorError> {
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
