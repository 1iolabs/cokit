// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{use_co_error, CoContext, CoError, CoErrorSignal};
use anyhow::anyhow;
use co_sdk::{Application, CoId, CoReducerState};
use dioxus::prelude::*;
use futures::{pin_mut, StreamExt};

/// Use state/heads from an CO.
pub fn use_co_state(co: &str) -> Signal<CoReducerState, SyncStorage> {
	// hooks
	let error = use_co_error();
	let state = use_signal_sync(Default::default);
	let context: CoContext = use_context();
	let hook = use_hook(|| {
		// run and update until drop_rx dropped
		let (tx, rx) = tokio::sync::watch::channel(CoId::from(co));
		let state_result = state;
		context.execute_future_parallel(move |application| async move {
			fetch_and_observe_state(application, rx, state_result, error).await;
		});
		CoStateHook { arguments: tx }
	});

	// update
	hook.update(co);

	// result
	state
}

#[derive(Clone)]
struct CoStateHook {
	arguments: tokio::sync::watch::Sender<CoId>,
}
impl CoStateHook {
	fn update(&self, next_co: &str) {
		self.arguments.send_if_modified(|co| {
			if co.as_str() != next_co {
				*co = CoId::from(next_co);
				true
			} else {
				false
			}
		});
	}
}

async fn fetch_and_observe_state(
	application: Application,
	mut arguments: tokio::sync::watch::Receiver<CoId>,
	mut state: Signal<CoReducerState, SyncStorage>,
	mut error: CoErrorSignal,
) {
	'arguments: loop {
		let co_id = arguments.borrow_and_update().clone();
		let reducer = application.co_reducer(&co_id).await;
		match reducer {
			Ok(Some(reducer)) => {
				// watch
				//  note: watch will immediately fire for initial event
				let stream = reducer.reducer_state_stream();
				pin_mut!(stream);
				tracing::info!(co = ?co_id, "watch");
				loop {
					tokio::select! {
						item = stream.next() => {
							match item {
								Some(next) => {
									tracing::info!(co = ?co_id, "watch-changed");
									if !next.is_empty() {
										tracing::info!(co = ?co_id, ?next, "watch-apply");
										*state.write() = next;
									}
								},
								None => {
									tracing::info!(co = ?co_id, "watch-failed");
									*state.write() = Default::default();
									error.write().push(CoError::from_error(anyhow!("Co not found: {}", co_id)));
									break;
								}
							}
						},
						item = arguments.changed() => {
							match item {
								Ok(_) => {
									// resubscribe
									tracing::info!(co = ?co_id, "watch-arguments");
									continue 'arguments;
								},
								Err(_) => {
									// done
									tracing::info!(co = ?co_id, "watch-dropped");
									return;
								},
							}
						},
					};
				}
			},
			Ok(None) => {
				*state.write() = Default::default();
				error.write().push(CoError::from_error(anyhow!("Co not found: {}", co_id)));
			},
			Err(err) => {
				*state.write() = Default::default();
				error.write().push(CoError::from_error(err));
			},
		}
	}
}
