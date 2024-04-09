use crate::{CoContext, CoStateResult};
use co_sdk::{Application, CoId, CoReducer, CoStorage, OptionLink};
use dioxus::prelude::*;
use futures::{pin_mut, Future, StreamExt};
use libipld::Cid;
use serde::de::DeserializeOwned;

pub fn use_co_selector<T, F, Fut>(co: &str, selector: F) -> Signal<CoStateResult<T>, SyncStorage>
where
	T: DeserializeOwned + Send + Sync + Default + Clone + 'static,
	F: Fn(CoStorage, Option<Cid>) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
{
	let co_id: CoId = co.into();
	let (drop_tx, drop_rx) = tokio::sync::oneshot::channel();

	// hooks
	let state = use_signal_sync(|| CoStateResult::Pending);
	let state_result = state;
	let context: CoContext = use_context();
	use_hook(move || {
		// run and update until drop_rx dropped
		context.execute(move |application| {
			tokio::spawn(fetch_and_observe_state(application.clone(), co_id, state, drop_rx, selector));
		});
		CoStateHook { done: Some(drop_tx) }
	});

	// result
	state_result
}

struct CoStateHook {
	done: Option<tokio::sync::oneshot::Sender<()>>,
}
impl Drop for CoStateHook {
	fn drop(&mut self) {
		if let Some(done) = self.done.take() {
			done.send(()).ok();
		}
	}
}
impl Clone for CoStateHook {
	fn clone(&self) -> Self {
		Self { done: None }
	}
}

async fn fetch_and_observe_state<T, F, Fut>(
	application: Application,
	co_id: CoId,
	mut state: Signal<CoStateResult<T>, SyncStorage>,
	mut drop_rx: tokio::sync::oneshot::Receiver<()>,
	selector: F,
) where
	T: DeserializeOwned + Send + Sync + Default + Clone + 'static,
	F: Fn(CoStorage, Option<Cid>) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
{
	let reducer = application.co_reducer(&co_id).await;
	match reducer {
		Ok(Some(reducer)) => {
			let mut read = StateReader::default();

			// initial
			read.read(&reducer, reducer.reducer_state().await.0.into(), state, &selector)
				.await;

			// watch
			let stream = reducer.observable().await.stream();
			pin_mut!(stream);
			loop {
				tokio::select! {
					_ = &mut drop_rx => {
						return;
					},
					item = stream.next() => {
						match item {
							Some((next_state, _)) => {
								read.read(&reducer, next_state.into(), state, &selector).await;
							},
							None => {
								// should not happen?
								*state.write() = CoStateResult::Error(format!("Co has been closed"));
								break;
							}
						}
					},
				};
			}
		},
		Ok(None) => {
			*state.write() = CoStateResult::Error(format!("Co not found: {}", co_id));
		},
		Err(err) => {
			*state.write() = CoStateResult::Error(format!("{}", err));
		},
	}
}

#[derive(Debug, Default)]
struct StateReader {
	last_state: OptionLink<co_core_co::Co>,
}
impl StateReader {
	async fn read<T, F, Fut>(
		&mut self,
		reducer: &CoReducer,
		next_state: OptionLink<co_core_co::Co>,
		mut state: Signal<CoStateResult<T>, SyncStorage>,
		selector: &F,
	) where
		T: DeserializeOwned + Send + Sync + Default + Clone + 'static,
		F: Fn(CoStorage, Option<Cid>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
	{
		if self.last_state != next_state {
			match selector(reducer.storage(), *next_state.cid()).await {
				Ok(result) => {
					self.last_state = next_state;
					*state.write() = CoStateResult::State(*next_state.cid(), result);
				},
				Err(err) => {
					*state.write() = CoStateResult::Error(format!("{}", err));
				},
			}
		}
	}
}
