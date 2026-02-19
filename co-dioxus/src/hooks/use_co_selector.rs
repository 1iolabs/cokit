// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{CoContext, CoStateResult};
use cid::Cid;
use co_sdk::{Application, CoId, CoReducer, CoStorage, OptionLink};
use dioxus::prelude::*;
use futures::{pin_mut, Future, StreamExt};

/// Select state from an CO.
pub fn use_co_selector<T, F, Fut, D>(co: &str, dependency: D, selector: F) -> Signal<CoStateResult<T>, SyncStorage>
where
	T: Send + Sync + Clone + 'static,
	F: Fn(CoStorage, Option<Cid>, D) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
	D: PartialEq + Clone + Send + Sync + 'static,
{
	// hooks
	let state = use_signal_sync(|| CoStateResult::Pending);
	let context: CoContext = use_context();
	let hook = use_hook(|| {
		// run and update until drop_rx dropped
		let (tx, rx) = tokio::sync::watch::channel((CoId::from(co), dependency.clone()));
		let state_result = state;
		context.execute_future_parallel(move |application| async move {
			fetch_and_observe_state(application, rx, state_result, selector).await;
		});
		CoStateHook { arguments: tx }
	});

	// update
	hook.update(co, dependency);

	// result
	state
}

#[derive(Clone)]
struct CoStateHook<D> {
	arguments: tokio::sync::watch::Sender<(CoId, D)>,
}
impl<D: PartialEq + Clone + Send + Sync + 'static> CoStateHook<D> {
	fn update(&self, next_co: &str, next_dependency: D) {
		self.arguments.send_if_modified(|(co, dependency)| {
			if co.as_str() != next_co || dependency != &next_dependency {
				*co = CoId::from(next_co);
				*dependency = next_dependency;
				true
			} else {
				false
			}
		});
	}
}

async fn fetch_and_observe_state<T, F, Fut, D>(
	application: Application,
	mut arguments: tokio::sync::watch::Receiver<(CoId, D)>,
	mut state: Signal<CoStateResult<T>, SyncStorage>,
	selector: F,
) where
	T: Send + Sync + Clone + 'static,
	F: Fn(CoStorage, Option<Cid>, D) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
	D: PartialEq + Clone + Send + Sync + 'static,
{
	'arguments: loop {
		let (co_id, dependency) = arguments.borrow_and_update().clone();
		let reducer = application.co_reducer(&co_id).await;
		match reducer {
			Ok(Some(reducer)) => {
				let mut read = StateReader::default();

				// watch
				//  note: watch will immediately fire for initial event
				let stream = reducer.reducer_state_stream();
				pin_mut!(stream);
				tracing::trace!(co = ?co_id, "watch");
				loop {
					tokio::select! {
						item = stream.next() => {
							match item {
								Some(next) => {
									tracing::trace!(co = ?co_id, "watch-changed");
									if let Some(next_state) = next.state() {
										tracing::trace!(co = ?co_id, ?next_state, "watch-apply");
										read.read(&reducer, next_state.into(), state, dependency.clone(), &selector).await;
									}
								},
								None => {
									tracing::trace!(co = ?co_id, "watch-failed");
									// should not happen?
									*state.write() = CoStateResult::Error("Co has been closed".to_string());
									break;
								}
							}
						},
						item = arguments.changed() => {
							match item {
								Ok(_) => {
									// resubscribe
									tracing::trace!(co = ?co_id, "watch-arguments");
									continue 'arguments;
								},
								Err(_) => {
									// done
									tracing::trace!(co = ?co_id, "watch-dropped");
									return;
								},
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
}

#[derive(Debug, Default)]
struct StateReader {
	last_state: OptionLink<co_core_co::Co>,
}
impl StateReader {
	async fn read<T, F, Fut, D>(
		&mut self,
		reducer: &CoReducer,
		next_state: OptionLink<co_core_co::Co>,
		mut state: Signal<CoStateResult<T>, SyncStorage>,
		dependency: D,
		selector: &F,
	) where
		T: Send + Sync + Clone + 'static,
		F: Fn(CoStorage, Option<Cid>, D) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<T, anyhow::Error>> + Send + 'static,
		D: PartialEq + Clone + Send + Sync + 'static,
	{
		if self.last_state != next_state {
			match selector(reducer.storage(), *next_state.cid(), dependency).await {
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
