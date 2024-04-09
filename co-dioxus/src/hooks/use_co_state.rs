use crate::{CoContext, CoStateResult};
use co_sdk::{state::core_state, Application, CoId, CoReducer, OptionLink};
use dioxus::prelude::*;
use futures::{pin_mut, StreamExt};
use serde::de::DeserializeOwned;

pub fn use_co_state<T>(co: &str, core: &str) -> Signal<CoStateResult<T>, SyncStorage>
where
	T: DeserializeOwned + Send + Sync + Default + Clone + 'static,
{
	let (drop_tx, drop_rx) = tokio::sync::oneshot::channel();
	let co_id: CoId = co.into();
	let core: String = core.to_string();

	// hooks
	let state = use_signal_sync(|| CoStateResult::Pending);
	let state_result = state;
	let context: CoContext = use_context();
	use_hook(move || {
		// run and update until drop_rx dropped
		context.execute(move |application| {
			tokio::spawn(fetch_and_observe_state(application.clone(), co_id, core, state, drop_rx));
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

async fn fetch_and_observe_state<T>(
	application: Application,
	co_id: CoId,
	core: String,
	mut state: Signal<CoStateResult<T>, SyncStorage>,
	mut drop_rx: tokio::sync::oneshot::Receiver<()>,
) where
	T: DeserializeOwned + Send + Sync + Default + Clone + 'static,
{
	let reducer = application.co_reducer(&co_id).await;
	match reducer {
		Ok(Some(reducer)) => {
			let mut read = StateReader::default();

			// initial
			read.read(&reducer, &core, reducer.reducer_state().await.0.into(), state).await;

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
								read.read(&reducer, &core, next_state.into(), state).await;
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
	async fn read<T: DeserializeOwned + Send + Sync + Default + Clone + 'static>(
		&mut self,
		reducer: &CoReducer,
		core: &str,
		next_state: OptionLink<co_core_co::Co>,
		mut state: Signal<CoStateResult<T>, SyncStorage>,
	) {
		if self.last_state != next_state {
			match core_state(&reducer.storage(), next_state, core).await {
				Ok((_, result)) => {
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
