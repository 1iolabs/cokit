// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::{
		co_actor::{CoActor, CoMessage},
		co_attachment::CoAttachment,
	},
	use_co_context, CoBlockStorage, CoContext, CoError,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::{Actor, ActorHandle};
use co_core_co::CoAction;
use co_sdk::{
	state::Identity, tags, unixfs_add, Application, CoId, CoReducerState, CreateCo, Tags, CO_CORE_NAME_CO, CO_ID_LOCAL,
};
use dioxus::prelude::*;
use futures::{future::Either, io::Cursor};
use serde::Serialize;
use std::{cell::RefCell, fmt::Debug, rc::Rc};

/// Use a single CO.
///
/// The requested CO is read on every render, so a component that re-renders with another id is
/// served the new CO in that very render while the previous one is detached.
pub fn use_co(co: ReadSignal<CoId>) -> Co {
	let context = use_co_context();
	let co_id = co();
	let mounted = use_hook({
		let context = context.clone();
		let co_id = co_id.clone();
		move || Rc::new(RefCell::new(MountedCo::attach(&context, co_id)))
	});

	// reconcile before returning so the current render never sees the previous CO again
	let mut mounted = mounted.borrow_mut();
	if mounted.co().co_id != co_id {
		*mounted = MountedCo::attach(&context, co_id);
	}
	mounted.co().clone()
}

/// A CO occurrence owned by a hook.
///
/// Dropping it requests shutdown of the CO actor, so a replaced, removed or unmounted occurrence
/// stops its background work even while public [`Co`] clones are still around.
///
/// Note: It is not cloneable by design: only the hook state may run the shutdown
pub(crate) struct MountedCo(Co);
impl MountedCo {
	pub(crate) fn attach(context: &CoContext, co_id: CoId) -> Self {
		Self(Co::attach(context.clone(), co_id))
	}

	pub(crate) fn co(&self) -> &Co {
		&self.0
	}
}
impl Drop for MountedCo {
	fn drop(&mut self) {
		self.0.handle.shutdown();
	}
}

#[derive(Debug, Clone)]
pub struct Co {
	pub(crate) co_id: CoId,
	pub(crate) context: CoContext,
	/// Owner of `reducer_state` and `last_error`. Background work keeps a clone so its late writes
	/// stay valid after this CO was detached.
	pub(crate) attachment: CoAttachment,
	pub(crate) reducer_state: SyncSignal<Option<Result<CoReducerState, CoError>>>,
	pub(crate) last_error: SyncSignal<Result<(), CoError>>,
	pub(crate) handle: ActorHandle<CoMessage>,
	pub(crate) storage: CoBlockStorage,
}
impl Co {
	/// Attach to a CO with a freshly created actor, storage handle and render state.
	pub(crate) fn attach(context: CoContext, co_id: CoId) -> Self {
		let attachment = CoAttachment::new();
		let actor_spawner = Actor::spawner(Default::default(), CoActor::new(co_id.clone())).expect("actor");
		let handle = actor_spawner.handle();
		context.execute_future_parallel({
			let attachment = attachment.clone();
			move |application| async move {
				actor_spawner.spawn(application.context().tasks(), (application.context().clone(), attachment));
			}
		});
		let storage = CoBlockStorage::new(handle.clone(), None);
		Co {
			co_id,
			context,
			reducer_state: attachment.reducer_state,
			last_error: attachment.last_error,
			attachment,
			handle,
			storage,
		}
	}

	pub fn co(&self) -> CoId {
		self.co_id.clone()
	}

	pub fn storage(&self) -> CoBlockStorage {
		self.storage.clone()
	}

	pub async fn reducer_state(&self) -> Result<CoReducerState, CoError> {
		Ok(self.handle.request(CoMessage::ReducerState).await?)
	}

	pub fn last_error(&self) -> Result<(), RenderError> {
		self.last_error.cloned().map_err(RenderError::from)
	}

	pub fn clear_last_error(&mut self) {
		self.last_error.set(Ok(()));
	}

	/// Push a action into a Co.
	///
	/// Use within [`dioxus::prelude::use_action`].
	pub async fn push<T>(
		&self,
		identity: Identity,
		core: impl Into<String> + Debug,
		action: T,
	) -> Result<CoReducerState, CoError>
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let co = self.co_id.clone();
		let core = core.into();
		self.context
			.try_with_application(move |application| async move {
				dispatch(application, identity, &co, &core, &action).await
			})
			.await
			.map_err(CoError::new)
	}

	/// Dispatch a action into a Co.
	///
	/// Note: Users should preferr [`Co::push`] with [`dioxus::prelude::use_action`] for more ergonmic error handling.
	pub fn dispatch<T>(&self, identity: Identity, core: impl Into<String> + Debug, action: T)
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let co = self.co_id.clone();
		let core = core.into();
		let attachment = self.attachment.clone();
		self.context.execute_future(move |application| async move {
			match dispatch(application, identity, &co, &core, &action).await {
				Ok(_) => {},
				Err(err) => {
					attachment.set_last_error(err.into());
				},
			}
		});
	}

	/// Create a new Co.
	pub fn create_co(&self, identity: Identity, co: CreateCo) {
		// check
		if self.co_id.as_str() != CO_ID_LOCAL {
			self.attachment
				.set_last_error(anyhow!("Create COs only support for local").into());
			return;
		}

		// create
		let attachment = self.attachment.clone();
		self.context.execute_future(move |application| async move {
			match create_co(application, identity, co).await {
				Ok(()) => {},
				Err(err) => {
					attachment.set_last_error(err.into());
				},
			}
		});
	}

	/// Create a core.
	pub fn create_core(&self, identity: Identity, core_name: &str, core_type: &str, core_binary: Cid) {
		let co = self.co_id.clone();
		let core_name = core_name.to_owned();
		let core_tags = tags!("type": core_type);
		let core_binary = Either::Left(core_binary);
		let attachment = self.attachment.clone();
		self.context.execute_future(move |application| async move {
			match create_core(application, identity, co, core_name, core_tags, core_binary).await {
				Ok(()) => {},
				Err(err) => {
					attachment.set_last_error(err.into());
				},
			}
		});
	}

	/// Create a core using binary.
	pub fn create_core_binary(
		&self,
		identity: Identity,
		core_name: &str,
		core_type: &str,
		core_binary: impl Into<Vec<u8>>,
	) {
		let co = self.co_id.clone();
		let core_name = core_name.to_owned();
		let core_tags = tags!("type": core_type);
		let core_binary = Either::Right(core_binary.into());
		let attachment = self.attachment.clone();
		self.context.execute_future(move |application| async move {
			match create_core(application, identity, co, core_name, core_tags, core_binary).await {
				Ok(()) => {},
				Err(err) => {
					attachment.set_last_error(err.into());
				},
			}
		});
	}
}
impl PartialEq for Co {
	fn eq(&self, other: &Self) -> bool {
		self.co_id == other.co_id
	}
}

async fn dispatch<T>(
	application: Application,
	identitiy: Identity,
	co: &CoId,
	core: &str,
	item: &T,
) -> Result<CoReducerState, anyhow::Error>
where
	T: Serialize + Debug + Send + Sync + Clone + 'static,
{
	let private_identity = application.private_identity(&identitiy.did).await?;
	let reducer = application
		.co_reducer(co)
		.await?
		.ok_or_else(|| anyhow::anyhow!("Co not found: {}", co))?;
	reducer.push(&private_identity, core, item).await
}

async fn create_co(application: Application, identitiy: Identity, co: CreateCo) -> Result<(), anyhow::Error> {
	let private_identity = application.private_identity(&identitiy.did).await?;
	application.create_co(private_identity, co).await?;
	Ok(())
}

async fn create_core(
	application: Application,
	identitiy: Identity,
	co: CoId,
	core_name: String,
	core_tags: Tags,
	core_binary: Either<Cid, Vec<u8>>,
) -> Result<(), anyhow::Error> {
	let private_identity = application.private_identity(&identitiy.did).await?;

	// reducer
	let reducer = application
		.co_reducer(&co)
		.await?
		.ok_or_else(|| anyhow::anyhow!("Co not found: {}", co))?;
	let storage = reducer.storage();

	// binary
	let binary = match core_binary {
		Either::Left(cid) => cid,
		Either::Right(bytes) => {
			let mut binary_stream = Cursor::new(&bytes);
			let binary = unixfs_add(&storage, &mut binary_stream)
				.await?
				.pop()
				.ok_or(anyhow!("Add Core binary failed {}", bytes.len()))?;
			binary
		},
	};

	// create
	reducer
		.push(&private_identity, CO_CORE_NAME_CO, &CoAction::CoreCreate { core: core_name, binary, tags: core_tags })
		.await?;

	Ok(())
}
