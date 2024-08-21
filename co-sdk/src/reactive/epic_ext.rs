use crate::Epic;
use futures::stream::BoxStream;
use tokio_stream::StreamExt;

pub trait EpicExt<A, S, C>: Epic<A, S, C>
where
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
{
	fn with<E>(self, other: E) -> JoinEpic<Self, E>
	where
		Self: Sized,
	{
		JoinEpic(self, other)
	}
}
impl<T, A, S, C> EpicExt<A, S, C> for T
where
	T: Epic<A, S, C> + ?Sized + Send + Sync + 'static,
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
{
}

pub struct JoinEpic<E1, E2>(E1, E2);
impl<E1, E2, A, S, C> Epic<A, S, C> for JoinEpic<E1, E2>
where
	A: Send + Clone + 'static,
	S: Send + Clone + 'static,
	C: Send + Clone + 'static,
	E1: Epic<A, S, C>,
	E2: Epic<A, S, C>,
{
	type Output = BoxStream<'static, A>;

	fn execute(&self, actions: crate::Observable<A>, state: crate::Observable<S>, context: C) -> Self::Output {
		let s1: <E1 as Epic<A, S, C>>::Output = self.0.execute(actions.clone(), state.clone(), context.clone());
		let s2 = self.1.execute(actions, state, context);
		Box::pin(s1.merge(s2))
	}
}
