use crate::{use_co_context, use_co_storage, CoBlockStorage, CoContext};
use co_sdk::{state::Identity, Application, CoId, CoReducerFactory, CoReducerState};
use dioxus::prelude::*;
use futures::{future::Either, pin_mut, StreamExt};
use serde::Serialize;
use std::{
	fmt::Debug,
	future::{ready, Future},
	rc::Rc,
	sync::Arc,
};
use tokio_util::sync::{CancellationToken, DropGuard};

pub fn use_co(co: ReadOnlySignal<CoId>) -> Co {
	// TODO: port storage
	let storage = use_co_storage(&co().to_string());
	let mut reducer_state = use_signal_sync(|| None);
	let last_error = use_signal_sync(|| Ok(()));
	let context = use_co_context();
	use_hook(move || {
		let co_id = co();
		let cancel = CancellationToken::new();
		let task = cancel.clone().drop_guard();
		context.execute_future_parallel({
			let co_id = co_id.clone();
			move |application| async move {
				let reducer = match application.co().try_co_reducer(&co_id).await {
					Ok(reducer) => reducer,
					Err(err) => {
						reducer_state.set(Some(Err(anyhow::Error::from(err).into())));
						return;
					},
				};
				let reducer_states = reducer.reducer_state_stream();
				pin_mut!(reducer_states);
				loop {
					tokio::select! {
						Some(next_state) = reducer_states.next() => {
							reducer_state.set(Some(Ok(next_state)));
						},
						_ = cancel.cancelled() => {
							break;
						},
						else => {
							break;
						}
					}
				}
			}
		});
		Co { co_id, last_error, context, _task: Rc::new(task), reducer_state, storage }
	})
}

pub fn use_selector<F, Fut, T>(co: &Co, f: F) -> Result<T, RenderError>
where
	F: Fn(CoBlockStorage, CoReducerState) -> Fut + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + 'static,
	T: Clone + 'static,
{
	let co = co.clone();
	let result = use_resource(move || {
		let fut = match co.try_reducer_state() {
			Ok(reducer_state) => {
				let fut = f(co.storage.clone(), reducer_state);
				Either::Left(async move { Ok(fut.await.map_err(CoError::from)?) })
			},
			Err(err) => Either::Right(ready(Err(err))),
		};
		async move { fut.await }
	})
	.suspend()?;
	result.cloned()
}

#[derive(Debug, Clone)]
pub struct Co {
	co_id: CoId,
	context: CoContext,
	reducer_state: SyncSignal<Option<Result<CoReducerState, CoError>>>,
	last_error: SyncSignal<Result<(), CoError>>,
	storage: CoBlockStorage,
	_task: Rc<DropGuard>,
}
impl Co {
	pub fn co(&self) -> CoId {
		self.co_id.clone()
	}

	pub fn storage(&self) -> CoBlockStorage {
		self.storage.clone()
	}

	pub fn try_reducer_state(&self) -> Result<CoReducerState, RenderError> {
		match self.reducer_state.cloned() {
			None => Err(RenderError::default()),
			Some(Ok(reducer_state)) => Ok(reducer_state),
			Some(Err(err)) => Err(err.into()),
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
