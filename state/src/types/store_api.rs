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
