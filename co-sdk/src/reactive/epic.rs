use crate::Observable;
use futures::Stream;

pub trait Epic<A, S, C>
where
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
{
	type Output: Stream<Item = A> + Send + 'static;

	fn execute(&self, actions: Observable<A>, state: Observable<S>, context: C) -> Self::Output;
}
impl<A, S, C, O, F> Epic<A, S, C> for F
where
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
	O: Stream<Item = A> + Send + 'static,
	F: Fn(Observable<A>, Observable<S>, C) -> O,
{
	type Output = O;

	fn execute(&self, actions: Observable<A>, state: Observable<S>, context: C) -> Self::Output {
		self(actions, state, context)
	}
}

#[cfg(test)]
mod tests {
	use crate::{Epic, Observable};
	use futures::{Stream, StreamExt};
	use std::future::ready;

	#[derive(Debug, Clone, PartialEq)]
	enum Action {
		A,
		B,
	}
	fn test_epic(actions: Observable<Action>, _state: Observable<()>, _context: ()) -> impl Stream<Item = Action> {
		actions.filter(|a| ready(matches!(a, Action::A))).map(|_| Action::B)
	}

	#[tokio::test]
	async fn smoke() {
		let actions = Observable::new();
		let states = Observable::new();
		let context = ();
		let (result, _) = futures::future::join(
			test_epic
				.execute(actions.clone(), states.clone(), context)
				.collect::<Vec<Action>>(),
			async move {
				actions.dispatch(Action::A);
				actions.shutdown();
			},
		)
		.await;
		assert_eq!(result, vec![Action::B]);
	}
}
