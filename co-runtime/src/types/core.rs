use co_api::{reduce_with_context, Context, Reducer};
use libipld::Cid;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

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
impl From<Cid> for Core {
	fn from(value: Cid) -> Self {
		Core::Wasm(value)
	}
}
