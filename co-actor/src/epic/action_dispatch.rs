// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Actions, TaskSpawner};
use futures::{channel::mpsc::UnboundedSender, Stream};
use std::future::Future;

/// Action Dispatch.
/// Encasulate epic logic in a single future.
pub struct ActionDispatch<A, S, C> {
	actions: Actions<A, S, C>,
	tx: UnboundedSender<Result<A, anyhow::Error>>,
}
impl<A, S, C> Clone for ActionDispatch<A, S, C> {
	fn clone(&self) -> Self {
		Self { actions: self.actions.clone(), tx: self.tx.clone() }
	}
}
impl<A, S, C> ActionDispatch<A, S, C>
where
	A: Clone + Send + 'static,
	S: Send + 'static,
	C: Send + 'static,
{
	/// Execute a stateful futures that dispatches/reacts to actions.
	pub fn execute<F, Fut>(
		actions: Actions<A, S, C>,
		spawner: TaskSpawner,
		f: F,
	) -> impl Stream<Item = Result<A, anyhow::Error>>
	where
		F: FnOnce(Self) -> Fut,
		Fut: Future<Output = Result<(), anyhow::Error>> + Send + 'static,
	{
		let (tx, rx) = futures::channel::mpsc::unbounded();
		let dispatch = ActionDispatch { actions, tx };
		let fut = f(dispatch.clone());
		spawner.spawn(async move {
			match fut.await {
				Ok(_) => {},
				Err(err) => {
					dispatch.dispatch_result(Err(err));
				},
			}
		});
		rx
	}

	/// Execute a stateful futures that dispatches/reacts to actions with an final result action.
	pub fn execute_with_response<F, Fut, R, O, E>(
		actions: Actions<A, S, C>,
		spawner: TaskSpawner,
		f: F,
		response: R,
	) -> impl Stream<Item = Result<A, anyhow::Error>>
	where
		F: FnOnce(Self) -> Fut,
		Fut: Future<Output = Result<O, E>> + Send + 'static,
		R: FnOnce(Result<O, E>) -> A + Send + 'static,
	{
		let (tx, rx) = futures::channel::mpsc::unbounded();
		let dispatch = ActionDispatch { actions, tx };
		let fut = f(dispatch.clone());
		spawner.spawn(async move {
			dispatch.dispatch(response(fut.await));
		});
		rx
	}

	/// Dispatch an action.
	/// Actions are dispatched immediately.
	pub fn dispatch(&self, item: A) -> bool {
		self.dispatch_result(Ok(item))
	}

	/// Dispatch an action result.
	/// Actions are dispatched immediately.
	pub fn dispatch_result(&self, item: Result<A, anyhow::Error>) -> bool {
		self.tx.unbounded_send(item).is_ok()
	}

	/// Request/Response.
	/// Dispatch `request` and wait for first `response`.
	pub async fn request<F, O>(&self, request: A, response: F) -> Result<O, anyhow::Error>
	where
		F: (for<'a> Fn(&'a A) -> Option<O>) + Clone + Send + 'static,
	{
		let response_fut = self.actions.once_map(response);
		self.dispatch(request);
		response_fut.await
	}
}
