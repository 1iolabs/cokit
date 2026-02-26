// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	library::co_actor::{CoActor, CoMessage},
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
use std::fmt::Debug;

pub fn use_co(co: ReadSignal<CoId>) -> Co {
	let reducer_state = use_signal_sync(|| None);
	let last_error = use_signal_sync(|| Ok(()));
	let context = use_co_context();
	use_hook(move || {
		let co_id = co();
		let actor_spawner = Actor::spawner(Default::default(), CoActor::new(co_id.clone())).expect("actor");
		let handle = actor_spawner.handle();
		context.execute_future_parallel(move |application| async move {
			actor_spawner.spawn(application.context().tasks(), (application.context().clone(), reducer_state));
		});
		let storage = CoBlockStorage::new(handle.clone(), None);
		Co { co_id, last_error, context, reducer_state, handle, storage }
	})
}

#[derive(Debug, Clone)]
pub struct Co {
	co_id: CoId,
	context: CoContext,
	pub(crate) reducer_state: SyncSignal<Option<Result<CoReducerState, CoError>>>,
	last_error: SyncSignal<Result<(), CoError>>,
	handle: ActorHandle<CoMessage>,
	storage: CoBlockStorage,
}
impl Co {
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
		let mut last_error = self.last_error;
		self.context.execute_future(move |application| async move {
			match dispatch(application, identity, &co, &core, &action).await {
				Ok(_) => {},
				Err(err) => {
					last_error.set(Err(err.into()));
				},
			}
		});
	}

	/// Create a new Co.
	pub fn create_co(&self, identity: Identity, co: CreateCo) {
		let mut last_error = self.last_error;

		// check
		if self.co_id.as_str() != CO_ID_LOCAL {
			last_error.set(Err(anyhow!("Create COs only support for local").into()));
			return;
		}

		// create
		self.context.execute_future(move |application| async move {
			match create_co(application, identity, co).await {
				Ok(()) => {},
				Err(err) => {
					last_error.set(Err(err.into()));
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
		let mut last_error = self.last_error;
		self.context.execute_future(move |application| async move {
			match create_core(application, identity, co, core_name, core_tags, core_binary).await {
				Ok(()) => {},
				Err(err) => {
					last_error.set(Err(err.into()));
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
		let mut last_error = self.last_error;
		self.context.execute_future(move |application| async move {
			match create_core(application, identity, co, core_name, core_tags, core_binary).await {
				Ok(()) => {},
				Err(err) => {
					last_error.set(Err(err.into()));
				},
			}
		});
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
