use crate::{actor::ActorMessage, ActorError, ActorHandle, ActorState, LocalTaskHandle, LocalTaskSpawner};
use co_primitives::Tags;
use std::{any::type_name, sync::Arc};
use tokio::sync::{mpsc, watch};
use tracing::Instrument;

/// A LocalActor will not moved between threads.
/// This is sometimes neccesarry when interfacing with external code.
/// This trait allows to implement such behaviour with same public interface as a normal [`Actor`] ([`ActorHandle`]).
/// For new code that dont have this requirement is usually better to use [`Actor`] as it allows to use multithreading.
#[allow(async_fn_in_trait)]
pub trait LocalActor: 'static {
	type Message: Send + 'static;
	type State: 'static;
	type Initialize: 'static;

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

	fn spawner(tags: Tags, actor: Self) -> Result<LocalActorSpawner<Self>, ActorError>
	where
		Self: Sized + 'static,
	{
		LocalActorSpawner::new(tags, actor)
	}

	/// Spawn actor using a task spawner.
	#[track_caller]
	fn spawn_with(
		spawner: impl LocalTaskSpawner,
		tags: Tags,
		actor: Self,
		initialize: Self::Initialize,
	) -> Result<LocalActorInstance<Self>, ActorError>
	where
		Self: Sized + 'static,
	{
		Ok(Self::spawner(tags, actor)?.spawn_local(spawner, initialize))
	}
}

/// Actor Spawner with early access to the handle (which allow cyclic references).
pub struct LocalActorSpawner<A>
where
	A: LocalActor,
{
	handle: ActorHandle<A::Message>,
	actor: A,
	rx: mpsc::UnboundedReceiver<ActorMessage<A::Message>>,
	state_tx: tokio::sync::watch::Sender<ActorState>,
}
impl<A> LocalActorSpawner<A>
where
	A: LocalActor,
{
	pub fn new(tags: Tags, actor: A) -> Result<Self, ActorError> {
		let (tx, rx) = mpsc::unbounded_channel();
		let (state_tx, state_rx) = watch::channel(ActorState::Starting);
		let tags = Arc::new(actor.tags(tags)?);
		let handle = ActorHandle { tx: tx.clone(), state: state_rx.clone(), tags: tags.clone() };
		Ok(Self { handle, actor, rx, state_tx })
	}

	pub fn handle(&self) -> ActorHandle<A::Message> {
		self.handle.clone()
	}

	#[track_caller]
	pub fn spawn_local(self, spawner: impl LocalTaskSpawner, initialize: A::Initialize) -> LocalActorInstance<A> {
		let mut rx = self.rx;
		let state_tx = self.state_tx;
		let actor = self.actor;
		let tags = self.handle.tags.clone();
		let handle = self.handle;
		let span = tracing::trace_span!("actor", ?tags, actor_type = type_name::<A>());
		let join = spawner.spawn_local({
			let tags = tags.clone();
			let handle = handle.clone();
			let actor_span = span.clone();
			async move {
				// log
				tracing::trace!(?tags, "actor-initialize");

				// initialize
				let mut actor_state = actor.initialize(&handle, &tags, initialize).await.map_err(|err| {
					tracing::error!(?err, ?tags, "actor-initialize-failed");
					err
				})?;
				state_tx
					.send(ActorState::Running)
					.map_err(|e| ActorError::InvalidState(e.into(), tags.as_ref().clone()))?;

				// execute
				let weak_handle = handle.downgrade();
				while let Some(actor_message) = rx.recv().await {
					// handle message
					let (message, message_span) = match actor_message {
						ActorMessage::Message(message) => (message, tracing::trace_span!("actor-handle")),
						ActorMessage::MessageWithSpan(message, message_span) => {
							(message, tracing::trace_span!(parent: message_span, "actor-handle"))
						},
						ActorMessage::Shutdown => {
							// log
							tracing::trace!(?tags, "actor-shutdown");

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
								tracing::error!(?err, ?tags, "actor-handle-failed");
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
		LocalActorInstance { handle, join }
	}
}

/// The actual actor instance.
pub struct LocalActorInstance<A>
where
	A: LocalActor,
{
	handle: ActorHandle<A::Message>,
	join: LocalTaskHandle<Result<(), ActorError>>,
}

impl<A: std::fmt::Debug> std::fmt::Debug for LocalActorInstance<A>
where
	A: LocalActor,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LocalActorInstance").field("handle", &self.handle).finish()
	}
}
impl<A> LocalActorInstance<A>
where
	A: LocalActor,
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

	/// Get actor state.
	pub fn state(&self) -> ActorState {
		*self.handle.state.borrow()
	}
}
