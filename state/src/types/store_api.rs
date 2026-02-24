// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{Middleware, Reducer};
use std::fmt::Debug;

/// Store API.
pub trait StoreApi<R: Reducer>: Debug {
	fn dispatch(&mut self, action: R::Action);
	fn state(&self) -> R::State;
	fn with_middleware(
		self: Box<Self>,
		middleware: Box<dyn Middleware<R> + Send + 'static>,
	) -> Box<dyn StoreApi<R> + Send + 'static>;
}
