// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{BoxEpic, Epic, EpicExt, MergeEpic};
use anyhow::anyhow;
use futures::{
	channel::oneshot,
	pin_mut,
	stream::{self},
	FutureExt, Stream, StreamExt,
};
use std::{
	future::{ready, Future},
	mem::take,
	ops::DerefMut,
	sync::{Arc, Mutex},
};

pub struct Actions<A, S, C> {
	pending: Arc<Mutex<Vec<BoxEpic<'static, A, S, C>>>>,
}
impl<A, S, C> Clone for Actions<A, S, C> {
	fn clone(&self) -> Self {
		Self { pending: self.pending.clone() }
	}
}
impl<A, S, C> Default for Actions<A, S, C> {
	fn default() -> Self {
		Self { pending: Arc::new(Mutex::new(Default::default())) }
	}
}
impl<A, S, C> Actions<A, S, C>
where
	A: Clone + Send + 'static,
{
	/// Wait once the epic emits its first action, remove the epic and return the action.
	/// This is guarantted to see all actions that are dispatched after this call has returned the future.
	pub fn once_epic<E>(&self, epic: E) -> impl Future<Output = Result<A, anyhow::Error>> + use<A, S, C, E>
	where
		E: EpicExt<A, S, C> + Send + 'static,
	{
		let (tx, rx) = oneshot::channel();

		// add
		{
			self.pending
				.lock()
				.unwrap()
				.push(OneshotEpic { epic, sender: Some(tx) }.boxed());
		}

		// wait
		async move { rx.await? }
	}

	/// Wait for predicate to match once and return the action it mached.
	/// This is guarantted to see all actions that are dispatched after this call has returned the future.
	pub fn once<F>(&self, predicate: F) -> impl Future<Output = Result<A, anyhow::Error>> + use<A, S, C, F>
	where
		F: for<'a> Fn(&'a A) -> bool + Send + 'static,
	{
		self.once_epic(FilterEpic(predicate))
	}

	/// Wait for map to match once and return the mapped value of the action.
	/// This is guarantted to see all actions that are dispatched after this call has returned the future.
	pub fn once_map<F, O>(&self, map: F) -> impl Future<Output = Result<O, anyhow::Error>> + use<A, S, C, F, O>
	where
		F: (for<'a> Fn(&'a A) -> Option<O>) + Clone + Send + 'static,
	{
		let action = self.once_epic(FilterEpic({
			let map = map.clone();
			move |action: &A| -> bool { map(action).is_some() }
		}));
		async move {
			let action = action.await?;
			map(&action).ok_or(anyhow!("Expected preficate to return some output"))
		}
	}
}

/// Action handle.
pub struct ActionsEpic<A, S, C> {
	inner: MergeEpic<A, S, C>,
	api: Actions<A, S, C>,
}
impl<A, S, C> Default for ActionsEpic<A, S, C> {
	fn default() -> Self {
		Self { inner: MergeEpic::new(), api: Default::default() }
	}
}
impl<A, S, C> ActionsEpic<A, S, C>
where
	A: Clone + Send + 'static,
{
	pub fn actions(&self) -> Actions<A, S, C> {
		self.api.clone()
	}
}
impl<A, S, C> Epic<A, S, C> for ActionsEpic<A, S, C>
where
	A: Send + 'static,
{
	fn epic(
		&mut self,
		actions: &Actions<A, S, C>,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static> {
		// move
		let pending = { take(self.api.pending.lock().unwrap().deref_mut()) };
		for item in pending {
			self.inner.box_push(item);
		}

		// execute
		let stream = self.inner.epic(actions, action, state, context).map(|s| s.boxed());

		// drain
		self.inner.drain_terminated();

		// result
		stream
	}
}

/// If predicate F matches emiit the action.
struct FilterEpic<F>(F);
impl<F, A, S, C> Epic<A, S, C> for FilterEpic<F>
where
	F: Fn(&A) -> bool + Send + 'static,
	A: Clone + Send + 'static,
{
	fn epic(
		&mut self,
		_actions: &Actions<A, S, C>,
		action: &A,
		_state: &S,
		_context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static> {
		if (self.0)(action) {
			Some(stream::iter([Ok(action.clone())]))
		} else {
			None
		}
	}
}

/// This epic will never emit but send to channel once the inner epic returns a stream.
struct OneshotEpic<E, A> {
	epic: E,
	sender: Option<oneshot::Sender<Result<A, anyhow::Error>>>,
}
impl<E, A, S, C> Epic<A, S, C> for OneshotEpic<E, A>
where
	E: Epic<A, S, C>,
	A: Clone + Send + 'static,
{
	fn epic(
		&mut self,
		actions: &Actions<A, S, C>,
		action: &A,
		state: &S,
		context: &C,
	) -> Option<impl Stream<Item = Result<A, anyhow::Error>> + Send + 'static> {
		if self.sender.is_some() {
			if let Some(stream) = self.epic.epic(actions, action, state, context) {
				if let Some(sender) = self.sender.take() {
					return Some(
						async move {
							pin_mut!(stream);
							if let Some(action) = stream.next().await {
								sender.send(action).ok();
							}
						}
						.into_stream()
						.filter_map(|_| ready(None)),
					);
				}
			}
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use crate::{epic::actions::ActionsEpic, Actions, Epic, EpicExt};
	use futures::{pin_mut, stream::select, FutureExt, Stream, TryStreamExt};

	#[derive(Debug, Clone, PartialEq)]
	enum TestAction {
		Greet,
		Hello,
		World,
	}
	struct Test {}
	impl Epic<TestAction, (), ()> for Test {
		fn epic(
			&mut self,
			actions: &Actions<TestAction, (), ()>,
			action: &TestAction,
			_state: &(),
			_context: &(),
		) -> Option<impl Stream<Item = Result<TestAction, anyhow::Error>> + Send + 'static> {
			match action {
				TestAction::Greet => Some({
					let actions = actions.clone();
					let answer_with_world = async move {
						let once_world = actions.once(|a| matches!(a, TestAction::Hello));

						// wait for world action
						once_world.await?;

						// greet
						Ok(TestAction::World)
					}
					.into_stream();
					let hello = async_stream::stream! { yield Ok(TestAction::Hello);};
					select(answer_with_world, hello)
				}),
				_ => None,
			}
		}
	}

	#[tokio::test]
	async fn test_once() {
		let actions_epic = ActionsEpic::default();
		let actions = actions_epic.actions();
		let test_epic = Test {};
		let mut epic = actions_epic.join(test_epic);
		let stream = epic.epic(&actions, &TestAction::Greet, &(), &()).expect("a stream");
		let mut result = Vec::new();
		pin_mut!(stream);
		while let Some(action) = stream.try_next().await.unwrap() {
			result.push(action.clone());
			if let Some(epic_actions) = epic.epic(&actions, &action, &(), &()) {
				let mut epic_actions = epic_actions.try_collect::<Vec<TestAction>>().await.unwrap();
				result.append(&mut epic_actions);
			}
		}
		assert_eq!(result, vec![TestAction::Hello, TestAction::World]);
	}
}
