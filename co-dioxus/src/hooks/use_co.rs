use crate::{use_co_context, use_co_storage, CoBlockStorage, CoContext};
use anyhow::anyhow;
use co_sdk::{state::Identity, Application, CoId, CoReducerFactory, CoReducerState, CreateCo, CO_ID_LOCAL};
use dioxus::prelude::*;
use futures::{future::Either, pin_mut, StreamExt};
use serde::Serialize;
use std::{fmt::Debug, future::Future, sync::Arc};

pub fn use_co(co: ReadOnlySignal<CoId>) -> Co {
	// TODO: port storage
	let storage = use_co_storage(&co().to_string());
	let mut reducer_state = use_signal_sync(|| None);
	let last_error = use_signal_sync(|| Ok(()));
	let context = use_co_context();
	use_hook(move || {
		let co_id = co();
		let (tx, rx) = tokio::sync::watch::channel(None);
		context.execute_future_parallel({
			let co_id = co_id.clone();
			move |application| async move {
				let reducer = match application.co().try_co_reducer(&co_id).await {
					Ok(reducer) => reducer,
					Err(err) => {
						reducer_state.set(Some(Err(anyhow::Error::from(err).into())));
						tx.send(reducer_state.cloned()).ok();
						return;
					},
				};
				let reducer_states = reducer.reducer_state_stream();
				pin_mut!(reducer_states);
				loop {
					tokio::select! {
						Some(next_state) = reducer_states.next() => {
							reducer_state.set(Some(Ok(next_state)));
							tx.send(reducer_state.cloned()).ok();
						},
						_ = tx.closed() => {
							break;
						},
						else => {
							break;
						}
					}
				}
			}
		});
		Co { co_id, last_error, context, reducer_state, reducer_state_watch: rx, storage }
	})
}

pub fn use_selector<F, Fut, T>(co: &Co, f: F) -> Result<T, RenderError>
where
	F: Fn(CoBlockStorage, CoReducerState) -> Fut + Clone + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	let result = use_resource({
		let co = co.clone();
		move || {
			let storage = co.storage.clone();
			let reducer_state = match co.reducer_state.cloned() {
				Some(reducer_state) => Either::Left(reducer_state),
				None => Either::Right(co.reducer_state_watch.clone()),
			};
			let f = f.clone();
			async move {
				let reducer_state = match reducer_state {
					Either::Left(reducer_state) => reducer_state?,
					Either::Right(reducer_state_watch) => lastest_reducer_state(reducer_state_watch).await?,
				};
				Result::<T, CoError>::Ok(f(storage, reducer_state).await?)
			}
		}
	})
	.suspend()?;
	Ok(result.cloned()?)
}

async fn lastest_reducer_state(
	mut reducer_state: tokio::sync::watch::Receiver<Option<Result<CoReducerState, CoError>>>,
) -> Result<CoReducerState, CoError> {
	loop {
		// done?
		match &*reducer_state.borrow_and_update() {
			Some(result) => {
				return result.clone();
			},
			None => {},
		}

		// wait for next change
		reducer_state.changed().await.map_err(anyhow::Error::from)?;
	}
}

#[derive(Debug, Clone)]
pub struct Co {
	co_id: CoId,
	context: CoContext,
	reducer_state: SyncSignal<Option<Result<CoReducerState, CoError>>>,
	reducer_state_watch: tokio::sync::watch::Receiver<Option<Result<CoReducerState, CoError>>>,
	last_error: SyncSignal<Result<(), CoError>>,
	storage: CoBlockStorage,
}
impl Co {
	pub fn co(&self) -> CoId {
		self.co_id.clone()
	}

	pub fn storage(&self) -> CoBlockStorage {
		self.storage.clone()
	}

	pub fn reducer_state(&self) -> Option<Result<CoReducerState, RenderError>> {
		match self.reducer_state.cloned() {
			None => None,
			Some(Ok(reducer_state)) => Some(Ok(reducer_state)),
			Some(Err(err)) => Some(Err(err.into())),
		}
	}

	pub fn last_error(&self) -> Result<(), RenderError> {
		self.last_error.cloned().map_err(|err| RenderError::from(err))
	}

	pub fn clear_last_error(&mut self) {
		self.last_error.set(Ok(()));
	}

	pub fn dispatch<T>(&self, identity: Identity, core: impl Into<String> + Debug, action: T)
	where
		T: Serialize + Debug + Send + Sync + Clone + 'static,
	{
		let co = self.co_id.clone();
		let core = core.into();
		let mut last_error = self.last_error;
		self.context.execute_future(move |application| async move {
			match dispatch(application, identity, &co, &core, &action).await {
				Ok(()) => {},
				Err(err) => {
					last_error.set(Err(err.into()));
				},
			}
		});
	}

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
}

#[derive(Clone, thiserror::Error)]
#[error(transparent)]
pub struct CoError(Arc<anyhow::Error>);
impl From<anyhow::Error> for CoError {
	fn from(err: anyhow::Error) -> Self {
		Self(Arc::new(err))
	}
}
impl Debug for CoError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

async fn dispatch<T>(
	application: Application,
	identitiy: Identity,
	co: &CoId,
	core: &str,
	item: &T,
) -> Result<(), anyhow::Error>
where
	T: Serialize + Debug + Send + Sync + Clone + 'static,
{
	let private_identity = application.private_identity(&identitiy.did).await?;
	let reducer = application
		.co_reducer(co)
		.await?
		.ok_or_else(|| anyhow::anyhow!("Co not found: {}", co))?;
	reducer.push(&private_identity, core, item).await?;
	Ok(())
}

async fn create_co(application: Application, identitiy: Identity, co: CreateCo) -> Result<(), anyhow::Error> {
	let private_identity = application.private_identity(&identitiy.did).await?;
	application.create_co(private_identity, co).await?;
	Ok(())
}
