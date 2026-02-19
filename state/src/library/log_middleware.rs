// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Middleware, Reducer};

pub struct LogMiddleware {}

impl LogMiddleware {
	pub fn new() -> Self {
		Self {}
	}
}

impl<R> Middleware<R> for LogMiddleware
where
	R: Reducer + 'static,
{
	fn dispatch<'a>(&mut self, next: &'a mut dyn crate::StoreApi<R>, action: R::Action) {
		// span
		tracing::span!(tracing::Level::INFO, "dispatch", ?action);

		// next
		next.dispatch(action);
	}
}
