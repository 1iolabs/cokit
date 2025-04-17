use super::{message::ReducerMessage, ReducerActor};
use crate::{
	library::create_reducer_action::{create_reducer_action, store_reducer_action},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{co_dispatch::CoDispatch, co_reducer_context::CoReducerContextRef, co_reducer_state::CoReducerState},
	CoStorage, DynamicCoDate, Reducer, Runtime,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorHandle, TaskSpawner};
use co_core_co::Co;
use co_identity::{PrivateIdentity, PrivateIdentityBox};
use co_primitives::{tags, BlockStorageSettings, CloneWithBlockStorageSettings, CoId, Link, ReducerAction};
use co_storage::{BlockStorageExt, StorageError};
use futures::Stream;
use ipld_core::ipld::Ipld;
use serde::Serialize;
use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData};

#[derive(Debug, Clone)]
pub struct CoReducer {
	id: CoId,
	parent: Option<CoId>,
	handle: ActorHandle<ReducerMessage>,
	storage: CoStorage,
	pub(crate) context: CoReducerContextRef,
	date: DynamicCoDate,
}
impl CoReducer {
	pub(crate) fn spawn(
		application_identifier: String,
		id: CoId,
		parent: Option<CoId>,
		storage: CoStorage,
		tasks: TaskSpawner,
		runtime: Runtime,
		reducer: Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		context: CoReducerContextRef,
	) -> Result<Self, anyhow::Error> {
		let date = reducer.date().clone();
		let actor = Actor::spawn_with(
			tasks.clone(),
			tags!("application": application_identifier, "co": id.as_str()),
			ReducerActor::new(tasks, runtime, context.clone()),
			reducer,
		)?;
		Ok(Self { id, parent, storage, handle: actor.handle(), context, date })
	}

	pub(crate) fn clone_with_detached_storage(&self) -> Self {
		self.clone_with_settings(BlockStorageSettings::new().with_detached())
	}

	pub(crate) fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		Self {
			id: self.id.clone(),
			parent: self.parent.clone(),
			handle: self.handle.clone(),
			context: self.context.clone(),
			date: self.date.clone(),
			storage: self.storage.clone_with_settings(settings),
		}
	}

	pub(crate) async fn clear(&self) -> CoReducerState {
		self.handle.request(ReducerMessage::Clear).await.unwrap_or_default()
	}

	pub(crate) fn handle(&self) -> ActorHandle<ReducerMessage> {
		self.handle.clone()
	}

	pub fn id(&self) -> &CoId {
		&self.id
	}

	pub fn date(&self) -> &DynamicCoDate {
		&self.date
	}

	pub fn parent_id(&self) -> Option<&CoId> {
		self.parent.as_ref()
	}

	/// Get current reducer heads.
	pub async fn heads(&self) -> BTreeSet<Cid> {
		self.reducer_state().await.1
	}

	/// Read co core state.
	pub async fn co(&self) -> Result<(CoStorage, Co), StorageError> {
		let storage = self.storage();
		let co = storage.get_value(&self.reducer_state().await.co()).await?;
		Ok((storage, co))
	}

	/// Get current reducer state and heads.
	pub async fn reducer_state(&self) -> CoReducerState {
		self.handle.request(ReducerMessage::State).await.unwrap_or_default()
	}

	/// Get a new storage instance for this CO.
	pub fn storage(&self) -> CoStorage {
		self.storage.clone()
	}

	/// Get reducer change stream. Upon start the current state is yielded.
	pub fn reducer_state_stream(&self) -> impl Stream<Item = CoReducerState> {
		self.handle.stream_graceful(ReducerMessage::StateStream)
	}

	/// Push event into reducer.
	///
	/// # Arguments
	/// - `identity` - The identity to sign the operation with.
	/// - `core` - The target core name. The key of [`co_core_co::Co::cores`].
	/// - `item` - The core action payload.
	///
	/// # Returns
	/// The resulting state and heads.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret, name = "push", fields(co = self.id().as_str(), identity = identity.identity()), skip(self, item, identity))]
	pub async fn push<T, I>(&self, identity: &I, core: &str, item: &T) -> Result<CoReducerState, anyhow::Error>
	where
		T: Serialize + Debug + Clone + Send + Sync + 'static,
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		let action_reference =
			create_reducer_action(&self.storage, identity, core, item, Default::default(), &self.date).await?;
		let result = self
			.handle
			.request(|r| {
				ReducerMessage::Push(
					PrivateIdentity::boxed(identity.clone()),
					self.storage.clone(),
					action_reference,
					r,
				)
			})
			.await??;
		tracing::trace!(action = ?item, ?action_reference, state = ?result.0, heads = ?result.1, "push");
		Ok(result)
	}

	/// Push event into reducer.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret, name = "push", fields(co = self.id().as_str(), identity = identity.identity(), core = action.core), skip(self, action, identity))]
	pub async fn push_action<T, I>(
		&self,
		identity: &I,
		action: &ReducerAction<T>,
	) -> Result<CoReducerState, anyhow::Error>
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		let action_reference = store_reducer_action(&self.storage, action, Default::default()).await?;
		let result = self
			.handle
			.request(|r| {
				ReducerMessage::Push(
					PrivateIdentity::boxed(identity.clone()),
					self.storage.clone(),
					action_reference,
					r,
				)
			})
			.await??;
		tracing::trace!(action = ?action.payload, ?action_reference, state = ?result.0, heads = ?result.1, "push");
		Ok(result)
	}

	/// Push event into reducer.
	///
	/// # Arguments
	/// - `identity` - The identity to sign the operation with.
	/// - `action_reference` - The reducer action reference.
	///
	/// # Returns
	/// The resulting state and heads.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret, name = "push", fields(co = self.id().as_str(), identity = identity.identity()), skip(self, identity))]
	pub async fn push_reference<I>(
		&self,
		identity: &I,
		action_reference: Link<ReducerAction<Ipld>>,
	) -> Result<CoReducerState, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		let result = self
			.handle
			.request(|r| {
				ReducerMessage::Push(
					PrivateIdentity::boxed(identity.clone()),
					self.storage.clone(),
					action_reference,
					r,
				)
			})
			.await??;
		tracing::trace!(?action_reference, state = ?result.0, heads = ?result.1, "push-reference");
		Ok(result)
	}

	/// Join heads.
	/// Returns true if state has changed.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret, fields(co = self.id().as_str()), skip(self))]
	pub async fn join(&self, heads: BTreeSet<Cid>) -> Result<CoReducerState, anyhow::Error> {
		Ok(self
			.handle
			.request(|r| ReducerMessage::JoinHeads(self.storage(), heads, r))
			.await??)
	}

	/// Join a previous (trusted) snapshot into history which may can used as a starting point.
	pub async fn join_state(&self, state: CoReducerState) -> Result<CoReducerState, anyhow::Error> {
		Ok(self
			.handle
			.request(|r| ReducerMessage::JoinState(self.storage(), state, r))
			.await??)
	}

	/// Create a action dispatcher.
	pub fn dispatcher<A, I>(&self, core: &str, identity: I) -> impl CoDispatch<A> + use<A, I>
	where
		A: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		CoReducerDispatch::new(self.clone(), PrivateIdentity::boxed(identity), core.to_string())
	}
}

struct CoReducerDispatch<A> {
	reducer: CoReducer,
	identity: PrivateIdentityBox,
	core: String,
	_action: PhantomData<A>,
}
impl<A> CoReducerDispatch<A> {
	fn new(reducer: CoReducer, identity: PrivateIdentityBox, core: String) -> Self {
		Self { reducer, identity, core, _action: PhantomData }
	}
}
#[async_trait]
impl<A> CoDispatch<A> for CoReducerDispatch<A>
where
	A: Serialize + Debug + Send + Sync + Clone + 'static,
{
	async fn dispatch(&self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		Ok(self.reducer.push(&self.identity, &self.core, action).await?.0)
	}
}
