use co_api::{reduce_with_context, Context, Reducer};
use libipld::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, sync::Arc};

/// A executable core reference.
#[derive(Clone)]
pub enum Core {
	Wasm(Cid),
	Native(Arc<dyn Fn(&mut dyn Context) + Send + Sync>),
}
impl Core {
	pub fn native<S>() -> Core
	where
		S: Reducer + Default + Serialize + DeserializeOwned,
		S::Action: DeserializeOwned,
	{
		Core::Native(Arc::new(|context| reduce_with_context::<S>(context)))
	}
}
impl Debug for Core {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Wasm(arg0) => f.debug_tuple("Wasm").field(arg0).finish(),
			Self::Native(_) => f.debug_tuple("Native").field(&"[native]").finish(),
		}
	}
}
impl From<Cid> for Core {
	fn from(value: Cid) -> Self {
		Core::Wasm(value)
	}
}
