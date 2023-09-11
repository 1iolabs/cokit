use crate::{Middleware, Reducer};
use rxrust::{prelude::Observer, subject::SubjectThreads};
use std::convert::Infallible;

/// Thread safe subject middleware.
///
/// Use Case: React to actions from outside.
pub struct SubjectMiddleware<T> {
	subject: SubjectThreads<T, Infallible>,
}

impl<T> SubjectMiddleware<T> {
	pub fn new(subject: SubjectThreads<T, Infallible>) -> Self {
		Self { subject }
	}
}

impl<R> Middleware<R> for SubjectMiddleware<R::Action>
where
	R: Reducer + 'static,
{
	fn dispatch<'a>(&mut self, next: &'a mut dyn crate::StoreApi<R>, action: <R as Reducer>::Action) {
		// next
		next.dispatch(action.clone());

		// epic
		self.subject.next(action);
	}
}
