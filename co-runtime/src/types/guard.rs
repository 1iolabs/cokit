use crate::AsyncContext;
use cid::Cid;
use co_api::{Guard, GuardRef};
use std::fmt::Debug;

/// A executable guard reference.
#[derive(Clone)]
pub enum GuardReference {
	Wasm(Cid),
	Binary(Vec<u8>),
	Native(GuardRef<AsyncContext>),
}
impl GuardReference {
	pub fn native<R>() -> GuardReference
	where
		R: Guard + 'static,
	{
		GuardReference::Native(GuardRef::new::<R>())
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
