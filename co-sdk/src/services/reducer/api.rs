// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{flush::CoReducerFlush, message::ReducerMessage, ReducerActor};
use crate::{
	application::memory::create_memory_reducer,
	library::create_reducer_action::{create_reducer_action, store_reducer_action},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{co_dispatch::CoDispatch, co_reducer_context::CoReducerContextRef, co_reducer_state::CoReducerState},
	ApplicationMessage, CoStorage, Reducer, Runtime, Storage,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorHandle, TaskSpawner};
use co_core_co::Co;
use co_identity::{PrivateIdentity, PrivateIdentityBox};
use co_primitives::{
	tags, BlockLinks, BlockStorageCloneSettings, CloneWithBlockStorageSettings, CoId, DynamicCoDate, Link, OptionLink,
	ReducerAction,
};
use co_storage::{
	BlockStorage, BlockStorageContentMapping, BlockStorageExt, ExtendedBlockStorage, LinksBlockStorage,
	OverlayBlockStorage, StorageError,
};
use futures::Stream;
use ipld_core::ipld::Ipld;
use serde::Serialize;
use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData};

/// CO handle.
#[derive(Debug, Clone)]
pub struct CoReducer {
	id: CoId,
	parent: Option<CoId>,
	handle: ActorHandle<ReducerMessage>,
	storage: CoStorage,
	overlay_storage: Option<OverlayBlockStorage<CoStorage>>,
	pub(crate) context: CoReducerContextRef,
	date: DynamicCoDate,
	runtime: Runtime,
	core_resolver: DynamicCoreResolver<CoStorage>,
	verify_links: Option<BlockLinks>,
}
impl CoReducer {
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn spawn(
		application_handle: ActorHandle<ApplicationMessage>,
		application_identifier: String,
		id: CoId,
		parent: Option<CoId>,
		tasks: TaskSpawner,
		runtime: Runtime,
		reducer: Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		context: CoReducerContextRef,
		flush: CoReducerFlush,
		initialize: bool,
		verify_links: Option<BlockLinks>,
	) -> Result<Self, anyhow::Error> {
		let date = reducer.date().clone();
		let core_resolver = reducer.core_resolver().clone();
		let storage = Self::create_storage(&verify_links, &context.storage(false));
		let actor = Actor::spawn_with(
			tasks.clone(),
			tags!("application": application_identifier, "co": id.as_str()),
			ReducerActor::new(id.clone(), runtime.clone(), application_handle, context.clone()),
			(initialize, storage.clone(), reducer, flush),
		)?;
		Ok(Self {
			id,
			parent,
			storage,
			handle: actor.handle(),
			context,
			date,
			overlay_storage: None,
			runtime,
			core_resolver,
			verify_links,
		})
	}

	/// Test if reducer is running.
	pub fn is_running(&self) -> bool {
		self.handle.is_running()
	}

	pub(crate) fn clone_with_detached_storage(&self) -> Self {
		self.clone_with_settings(BlockStorageCloneSettings::new().with_detached())
	}

	pub(crate) fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		let (storage, overlay_storage) = match &self.overlay_storage {
			Some(overlay_storage) => {
				// clone the base storage without overlay but with settings
				let storage = self.context.storage(false).clone_with_settings(settings);

				// clone the overlay and replace next storage with the cloned instance
				//  this will use the same overlay as the source clone
				let overlay_storage = overlay_storage.clone().with_next_storage(storage);

				// result
				(Self::create_storage(&self.verify_links, &overlay_storage), Some(overlay_storage))
			},
			None => (self.storage.clone_with_settings(settings), None),
		};
		Self {
			id: self.id.clone(),
			parent: self.parent.clone(),
			handle: self.handle.clone(),
			context: self.context.clone(),
			date: self.date.clone(),
			runtime: self.runtime.clone(),
			core_resolver: self.core_resolver.clone(),
			storage,
			overlay_storage,
			verify_links: self.verify_links.clone(),
		}
	}

	/// Use a new overlay storage for this instance.
	pub(crate) fn with_overlay_storage(mut self, tasks: TaskSpawner, storage: Storage) -> Self {
		let base_storage = self.context.storage(false);
		let overlay_storage = OverlayBlockStorage::new(tasks, base_storage, storage.tmp_storage(), None, true, true);
		self.storage = Self::create_storage(&self.verify_links, &overlay_storage);
		self.overlay_storage = Some(overlay_storage);
		self
	}

	/// Create the actural storage instance.
	fn create_storage<S>(verify_links: &Option<BlockLinks>, storage: &S) -> CoStorage
	where
		S: BlockStorage + ExtendedBlockStorage + BlockStorageContentMapping + CloneWithBlockStorageSettings + 'static,
	{
		if let Some(verify_links) = &verify_links {
			CoStorage::new(LinksBlockStorage::new(storage.clone(), Some(verify_links.clone())))
		} else {
			CoStorage::new(storage.clone())
		}
	}

	pub(crate) async fn clear(&self) -> CoReducerState {
		// clear overlay
		if let Some(overlay_storage) = &self.overlay_storage {
			overlay_storage.clear_overlay_changes().await.ok();
		}

		// clear
		self.handle.request(ReducerMessage::Clear).await.unwrap_or_default()
	}

	pub(crate) fn handle(&self) -> ActorHandle<ReducerMessage> {
		self.handle.clone()
	}

	/// Get the CoId.
	pub fn id(&self) -> &CoId {
		&self.id
	}

	/// Get the used date provider.
	pub fn date(&self) -> &DynamicCoDate {
		&self.date
	}

	/// Get the CoId of which this co is a member of.
	/// Usually this is the Local CO (`"local"`).
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

	/// Read co core state link.
	pub async fn co_state(&self) -> OptionLink<Co> {
		self.reducer_state().await.co()
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
	pub async fn push<T, I>(
		&self,
		identity: &I,
		core: impl Into<String> + Debug,
		item: &T,
	) -> Result<CoReducerState, anyhow::Error>
	where
		T: Serialize + Debug + Clone + Send + Sync + 'static,
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		// action
		let action_reference =
			create_reducer_action(&self.storage, identity, core, item, Default::default(), &self.date).await?;

		// push
		let result = self
			.handle
			.try_request(|r| {
				ReducerMessage::Push(
					self.overlay_storage.clone(),
					self.storage.clone(),
					PrivateIdentity::boxed(identity.clone()),
					action_reference,
					r,
				)
			})
			.await?;

		// result
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
		// action
		let action_reference = store_reducer_action(&self.storage, action, Default::default()).await?;

		// push
		let result = self
			.handle
			.try_request(|r| {
				ReducerMessage::Push(
					self.overlay_storage.clone(),
					self.storage.clone(),
					PrivateIdentity::boxed(identity.clone()),
					action_reference,
					r,
				)
			})
			.await?;

		// result
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
		// push
		let result = self
			.handle
			.try_request(|r| {
				ReducerMessage::Push(
					self.overlay_storage.clone(),
					self.storage.clone(),
					PrivateIdentity::boxed(identity.clone()),
					action_reference,
					r,
				)
			})
			.await?;

		// result
		tracing::trace!(?action_reference, state = ?result.0, heads = ?result.1, "push-reference");
		Ok(result)
	}

	/// Join heads.
	///
	/// # Concurrency
	/// This call will block the reducer only while the computed state is integrated.
	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), ret, fields(co = self.id().as_str()), skip(self))]
	pub async fn join(&self, heads: BTreeSet<Cid>) -> Result<CoReducerState, anyhow::Error> {
		// create reducer
		let (storage, mut reducer) = self.create_memory_reducer().await?;

		// to internal
		let heads = CoReducerState::new(None, heads).to_internal(&storage).await;

		// join
		Ok(if reducer.join(&storage, &heads.1, self.runtime.runtime()).await?.is_some() {
			// integrate join
			self.join_state(CoReducerState::new_reducer(&reducer)).await?
		} else {
			// no change
			self.reducer_state().await
		})
	}

	/// Join a previous (trusted) snapshot into history which may can used as a starting point.
	///
	/// # Concurrency
	/// This call will block the reducer until the join has been fully processed.
	pub async fn join_state(&self, state: CoReducerState) -> Result<CoReducerState, anyhow::Error> {
		// join
		let co_reducer_state = self
			.handle
			.try_request(|r| ReducerMessage::JoinState(self.overlay_storage.clone(), self.storage(), state, r))
			.await?;

		// result
		Ok(co_reducer_state)
	}

	/// Create a action dispatcher.
	pub fn dispatcher<A, I, C>(&self, core: C, identity: I) -> impl CoDispatch<A> + use<A, I, C>
	where
		A: Serialize + Debug + Send + Sync + Clone + 'static,
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
		C: Into<String> + Debug,
	{
		CoReducerDispatch::new(self.clone(), PrivateIdentity::boxed(identity), core.into())
	}

	/// Create a memory reducer with detached state/heads to precompute operations.
	///
	/// # Notes
	/// - Uses the same storage instance.
	/// - Uses the same core resolver instance.
	async fn create_memory_reducer(
		&self,
	) -> Result<(CoStorage, Reducer<CoStorage, DynamicCoreResolver<CoStorage>>), anyhow::Error> {
		let storage = self.storage();
		let reducer_state = self.reducer_state().await;
		let reducer = create_memory_reducer(
			self.runtime.runtime(),
			self.date.clone(),
			&self.id,
			&storage,
			Some(self.core_resolver.clone()),
			reducer_state,
		)
		.await?;
		Ok((storage, reducer))
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
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		Ok(self.reducer.push(&self.identity, &self.core, action).await?.0)
	}
}
