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
