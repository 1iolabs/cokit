// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

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
