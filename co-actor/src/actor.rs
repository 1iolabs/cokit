// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	Response, ResponseBackPressureStream, ResponseBackPressureStreamReceiver, ResponseReceiver, ResponseStream,
	ResponseStreamReceiver, TaskHandle, TaskOptions, TaskSpawner,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::Tags;
use futures::{Stream, StreamExt};
use std::{any::type_name, future::ready, ops::Deref, sync::Arc};
use tokio::sync::{mpsc, watch};
#[cfg(feature = "js")]
use tokio_with_wasm::alias as tokio;
use tracing::{Instrument, Span};

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
	#[error("Invalid actor state for that operation ({1}).")]
	InvalidState(#[source] anyhow::Error, Tags),

	#[error("Operation canceled.")]
	Canceled,

	#[error("Actor error")]
	Actor(#[from] anyhow::Error),
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
		tags: &Tags,
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

	/// Shutdown the actor.
	/// This is not cancelable.
	/// After this call no more message will be received.
	/// Will not be executed if actor panics.
	async fn shutdown(&self, _state: Self::State) -> Result<(), ActorError> {
		Ok(())
	}

	fn spawner(tags: Tags, actor: Self) -> Result<ActorSpawner<Self>, ActorError>
	where
		Self: Send + Sync + Sized + 'static,
	{
		ActorSpawner::new(tags, actor)
	}

	/// Spawn actor.
	#[track_caller]
	fn spawn(tags: Tags, actor: Self, initialize: Self::Initialize) -> Result<ActorInstance<Self>, ActorError>
	where
		Self: Send + Sync + Sized + 'static,
	{
		Self::spawn_with(Default::default(), tags, actor, initialize)
	}

	/// Spawn actor using a task spawner.
	/// TODO: to simplyfy lifecycle make async and wait for intitalize?
	#[track_caller]
	fn spawn_with(
		spawner: TaskSpawner,
		tags: Tags,
		actor: Self,
		initialize: Self::Initialize,
	) -> Result<ActorInstance<Self>, ActorError>
	where
		Self: Send + Sync + Sized + 'static,
	{
		Ok(Self::spawner(tags, actor)?.spawn(spawner, initialize))
	}
}

/// Actor Spawner with early access to the handle (which allow cyclic references).
pub struct ActorSpawner<A>
where
	A: Actor,
{
	handle: ActorHandle<A::Message>,
	actor: A,
	rx: tokio::sync::mpsc::UnboundedReceiver<ActorMessage<A::Message>>,
	state_tx: tokio::sync::watch::Sender<ActorState>,
	options: TaskOptions,
}
impl<A> ActorSpawner<A>
where
	A: Actor,
{
	pub fn new(tags: Tags, actor: A) -> Result<Self, ActorError> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let (state_tx, state_rx) = watch::channel(ActorState::Starting);
		let tags = Arc::new(actor.tags(tags)?);
		let handle = ActorHandle { tx: tx.clone(), state: state_rx.clone(), tags: tags.clone() };
		Ok(Self { handle, actor, rx, state_tx, options: TaskOptions::new(type_name::<A>()) })
	}

	pub fn handle(&self) -> ActorHandle<A::Message> {
		self.handle.clone()
	}

	#[track_caller]
	pub fn spawn(self, spawner: TaskSpawner, initialize: A::Initialize) -> ActorInstance<A> {
		let mut rx = self.rx;
		let state_tx = self.state_tx;
		let actor = self.actor;
		let tags = self.handle.tags.clone();
		let handle = self.handle;
		let span = tracing::trace_span!("actor", ?tags, actor_type = type_name::<A>());
		let join = spawner.spawn_options(self.options, {
			let tags = tags.clone();
			let handle = handle.clone();
			let actor_span = span.clone();
			async move {
				// log
				tracing::trace!("actor-initialize");

				// initialize
				let mut actor_state = actor.initialize(&handle, &tags, initialize).await.map_err(|err| {
					tracing::error!(?err, "actor-initialize-failed");
					err
				})?;
				state_tx
					.send(ActorState::Running)
					.map_err(|e| ActorError::InvalidState(e.into(), tags.as_ref().clone()))?;

				// execute
				let weak_handle = handle.downgrade();
				while let Some(actor_message) = rx.recv().await {
					let (message, message_span) = match actor_message {
						ActorMessage::Message(message) => (message, tracing::trace_span!("actor-handle")),
						ActorMessage::MessageWithSpan(message, message_span) => {
							(message, tracing::trace_span!(parent: message_span, "actor-handle"))
						},
						ActorMessage::Shutdown => {
							// log
							tracing::trace!("actor-shutdown");

							// done
							break;
						},
					};
					message_span.follows_from(&actor_span);

					// get a strong handle to call the handle method - this should never fail as we should not
					// receive any message when this fails.
					if let Some(handle) = weak_handle.clone().upgrade() {
						actor
							.handle(&handle, message, &mut actor_state)
							.instrument(message_span)
							.await
							.map_err(|err| {
								tracing::error!(?err, "actor-handle-failed");
								err
							})?;
					}
				}

				// state
				state_tx
					.send(ActorState::Stopping)
					.map_err(|e| ActorError::InvalidState(e.into(), tags.as_ref().clone()))?;
				rx.close();

				// shutdown
				actor.shutdown(actor_state).await.map_err(|err| {
					tracing::error!(?err, ?tags, "actor-shutdown-failed");
					err
				})?;

				// done
				state_tx
					.send(ActorState::None)
					.map_err(|e| ActorError::InvalidState(e.into(), tags.as_ref().clone()))?;
				Ok(())
			}
			.instrument(span)
		});
		ActorInstance { join, handle }
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
pub enum ActorMessage<M> {
	/// Actor shutdown requested.
	Shutdown,

	/// Actor received message.
	Message(M),

	/// Actor received message.
	MessageWithSpan(M, tracing::Span),
}

/// The actual actor instance.
#[derive(Debug)]
pub struct ActorInstance<A>
where
	A: Actor,
{
	handle: ActorHandle<A::Message>,
	join: TaskHandle<Result<(), ActorError>>,
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
		self.handle.tags.as_ref().clone()
	}

	/// Request shutdown.
	pub fn shutdown(&self) {
		self.handle().shutdown();
	}

	/// Wait until the actor completes.
	pub async fn join(self) -> Result<(), ActorError> {
		let tags = self.tags();
		drop(self.handle);
		self.join.await.map_err(|e| ActorError::InvalidState(e.into(), tags))??;
		Ok(())
	}

	/// Wait for startup to be complete and then run in background.
	/// This will resolve when initialization is done by returning any initialization errors.
	pub async fn initialized(self) -> Result<ActorHandle<A::Message>, ActorError> {
		let handle = self.handle();
		match handle.initialized().await {
			Ok(_) => Ok(handle),
			Err(err @ ActorError::InvalidState(_, _)) if self.handle().is_closed() => {
				// use the orignal initialize error and forward
				//  this will not block as the actor has been closed already
				self.join().await?;
				Err(err)
			},
			Err(err) => Err(err),
		}
	}

	/// Get actor state.
	pub fn state(&self) -> ActorState {
		*self.handle.state.borrow()
	}
}

/// Handle into an actor which can be used to send messages.
pub struct ActorHandle<M> {
	pub(crate) tx: mpsc::UnboundedSender<ActorMessage<M>>,
	pub(crate) state: watch::Receiver<ActorState>,
	pub(crate) tags: Arc<Tags>,
}
impl<M> std::fmt::Debug for ActorHandle<M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ActorHandle")
			.field("message_type", &type_name::<M>())
			.field("tx_closed", &self.tx.is_closed())
			.field("state", &self.state.borrow().deref())
			.field("tags", &self.tags)
			.finish()
	}
}
impl<M> Clone for ActorHandle<M> {
	fn clone(&self) -> Self {
		Self { tx: self.tx.clone(), state: self.state.clone(), tags: self.tags.clone() }
	}
}
impl<M> ActorHandle<M>
where
	M: Send + 'static,
{
	/// Convert to weak actor handle.
	pub fn downgrade(self) -> WeakActorHandle<M> {
		WeakActorHandle { state: self.state, tags: self.tags, tx: self.tx.downgrade() }
	}

	/// Get actor tags.
	pub fn tags(&self) -> &Tags {
		self.tags.as_ref()
	}

	/// Check if actor is running.
	///
	/// Running means not initializing, stopping or stopped.
	pub fn is_running(&self) -> bool {
		match *self.state.borrow() {
			ActorState::Running => !self.tx.is_closed(),
			_ => false,
		}
	}

	/// Check if actor is closed.
	///
	/// Closed means not initializing or running.
	pub fn is_closed(&self) -> bool {
		match *self.state.borrow() {
			ActorState::Stopping => true,
			_ => self.tx.is_closed(),
		}
	}

	/// Wait for startup to be complete.
	pub async fn initialized(&self) -> Result<(), ActorError> {
		let mut state = self.state.clone();
		loop {
			let actor_state = *state.borrow_and_update();
			match actor_state {
				ActorState::Starting => {
					state
						.changed()
						.await
						.map_err(|e| ActorError::InvalidState(e.into(), self.tags().clone()))?;
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
		actor_closed(self.state.clone(), self.tags.clone()).await
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
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running."), self.tags().clone()))?;
		Ok(())
	}

	/// Request with response.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip_all, fields(message_type = type_name::<M>()))]
	pub async fn request<T>(&self, message: impl FnOnce(Response<T>) -> M) -> Result<T, ActorError> {
		let (responder, response) = ResponseReceiver::new();
		self.tx
			.send(ActorMessage::MessageWithSpan(message(responder), Span::current()))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running."), self.tags().clone()))?;
		response.await
	}

	/// Request with response result.
	/// If an error is returned in the result it will be wrapped in ´ActorError::Actor`.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip_all, fields(message_type = type_name::<M>()))]
	pub async fn try_request<T, E>(&self, message: impl FnOnce(Response<Result<T, E>>) -> M) -> Result<T, ActorError>
	where
		E: Into<anyhow::Error>,
	{
		let (responder, response) = ResponseReceiver::new();
		self.tx
			.send(ActorMessage::MessageWithSpan(message(responder), Span::current()))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running."), self.tags().clone()))?;
		response
			.await?
			.map_err(|err| ActorError::Actor(err.into().context(anyhow!("Actor try request: {}", type_name::<M>()))))
	}

	/// Request with streaming response.
	///
	/// # Errors
	/// The stream only fails if the stream request could not be sent to the actor because it's not running.
	/// In this case [`ActorError::InvalidState`] is returned and the stream ends after it.
	pub fn stream<T>(&self, message: impl FnOnce(ResponseStream<T>) -> M) -> impl Stream<Item = Result<T, ActorError>> {
		let (responder, response) = ResponseStreamReceiver::new();
		let send_result = self
			.tx
			.send(ActorMessage::Message(message(responder)))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running."), self.tags().clone()));
		let handle = self.clone();
		async_stream::stream! {
			// force keep actor alive while stream is running
			let _handle = handle;

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
				yield Ok(item);
			}
		}
	}

	/// Request with streaming response.
	/// Gracefully ends the stream when the actor is not running.
	pub fn stream_graceful<T>(&self, message: impl FnOnce(ResponseStream<T>) -> M) -> impl Stream<Item = T> {
		self.stream(message).filter_map(|item| ready(item.ok()))
	}

	/// Request with streaming response wtih backpressure.
	pub fn stream_backpressure<T: std::fmt::Debug>(
		&self,
		buffer: usize,
		message: impl FnOnce(ResponseBackPressureStream<T>) -> M,
	) -> impl Stream<Item = Result<T, ActorError>> {
		let (responder, response) = ResponseBackPressureStreamReceiver::new(buffer);
		let send_result = self
			.tx
			.send(ActorMessage::Message(message(responder)))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running."), self.tags().clone()));
		let handle = self.clone();
		async_stream::stream! {
			// force keep actor alive while stream is running
			let _handle = handle;

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

#[derive(Debug)]
pub struct WeakActorHandle<M> {
	tx: mpsc::WeakUnboundedSender<ActorMessage<M>>,
	state: watch::Receiver<ActorState>,
	tags: Arc<Tags>,
}
impl<M> Clone for WeakActorHandle<M> {
	fn clone(&self) -> Self {
		Self { tx: self.tx.clone(), state: self.state.clone(), tags: self.tags.clone() }
	}
}
impl<M> WeakActorHandle<M> {
	pub fn upgrade(self) -> Option<ActorHandle<M>> {
		Some(ActorHandle { state: self.state, tags: self.tags, tx: self.tx.upgrade()? })
	}

	/// Wait for actor shutdown.
	pub async fn closed(&self) -> Result<(), ActorError> {
		actor_closed(self.state.clone(), self.tags.clone()).await
	}
}

/// Wait for actor shutdown.
async fn actor_closed(mut state: watch::Receiver<ActorState>, tags: Arc<Tags>) -> Result<(), ActorError> {
	loop {
		let actor_state = *state.borrow_and_update();
		match actor_state {
			ActorState::Starting | ActorState::Running => {
				state
					.changed()
					.await
					.map_err(|e| ActorError::InvalidState(e.into(), tags.as_ref().clone()))?;
			},
			_ => {
				break;
			},
		}
	}
	Ok(())
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
	use std::time::Duration;
	use tokio::time::timeout;

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
				_tags: &Tags,
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
						*state += value;
					},
					TestMessage::Get(response) => {
						response.respond(*state);
					},
					TestMessage::IncGet(value, response) => {
						*state += value;
						response.respond(*state);
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
				_tags: &Tags,
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
						state.value += value;
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

	#[tokio::test]
	async fn test_drop_when_no_handles() {
		struct Test {}
		enum TestMessage {
			Inc(i32),
			Get(Response<i32>),
		}
		#[async_trait]
		impl Actor for Test {
			type Message = TestMessage;
			type State = i32;
			type Initialize = i32;

			async fn initialize(
				&self,
				_handle: &ActorHandle<Self::Message>,
				_tags: &Tags,
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
						*state += value;
					},
					TestMessage::Get(response) => {
						response.send(*state).ok();
					},
				}
				Ok(())
			}
		}

		// spawn
		let actor = Actor::spawn(Default::default(), Test {}, 1).unwrap();

		// do some work
		let handle = actor.handle();
		handle.dispatch(TestMessage::Inc(10)).unwrap();
		assert_eq!(handle.request(TestMessage::Get).await.unwrap(), 11);

		// drop handle and wait for shutdown
		drop(handle);
		timeout(Duration::from_millis(100), actor.join()).await.unwrap().unwrap();
	}
}
