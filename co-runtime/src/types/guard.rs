use crate::AsyncContext;
use cid::Cid;
use co_api::{guard_with_context, Guard};
use std::{fmt::Debug, sync::Arc};

/// A executable guard reference.
#[derive(Clone)]
pub enum GuardReference {
	Wasm(Cid),
	Binary(Vec<u8>),
	Native(Arc<dyn Fn(AsyncContext) -> bool + Send + Sync>),
}
impl GuardReference {
	pub fn native<R>() -> GuardReference
	where
		R: Guard,
	{
		GuardReference::Native(Arc::new(|context| guard_with_context::<AsyncContext, R>(context)))
	}
}
impl Debug for GuardReference {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Wasm(arg0) => f.debug_tuple("Wasm").field(arg0).finish(),
			Self::Binary(arg0) => f.debug_tuple("Binary").field(&arg0.len()).finish(),
			Self::Native(_) => f.debug_tuple("Native").field(&"[native]").finish(),
		}
	}
}
impl From<Cid> for GuardReference {
	fn from(value: Cid) -> Self {
		GuardReference::Wasm(value)
	}
}
