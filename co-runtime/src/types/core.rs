use crate::{AsyncBlockStorage, AsyncContext};
use cid::Cid;
use co_api::{
	async_api,
	sync_api::{reduce_with_context, Context, Reducer},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, sync::Arc};

/// A executable core reference.
#[derive(Clone)]
pub enum Core {
	Wasm(Cid),
	Native(Arc<dyn Fn(&mut dyn Context) + Send + Sync>),
	NativeAsync(Arc<dyn Fn(AsyncContext) -> AsyncContext + Send + Sync>),
}
impl Core {
	pub fn native<S>() -> Core
	where
		S: Reducer + Default + Serialize + DeserializeOwned,
		S::Action: DeserializeOwned,
	{
		Core::Native(Arc::new(|context| reduce_with_context::<S>(context)))
	}

	pub fn native_async<R, A>() -> Core
	where
		R: async_api::Reducer<A, AsyncBlockStorage> + Default,
		A: Clone + DeserializeOwned,
	{
		Core::NativeAsync(Arc::new(|context| {
			async_api::reduce_with_context::<R, A, AsyncContext, AsyncBlockStorage>(context)
		}))
	}

	pub fn is_native(&self) -> bool {
		match &self {
			Core::Wasm(_) => false,
			Core::Native(_) => true,
			Core::NativeAsync(_) => true,
		}
	}
}
impl Debug for Core {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Wasm(arg0) => f.debug_tuple("Wasm").field(arg0).finish(),
			Self::Native(_) => f.debug_tuple("Native").field(&"[native]").finish(),
			Self::NativeAsync(_) => f.debug_tuple("NativeAsync").field(&"[native]").finish(),
		}
	}
}
impl From<Cid> for Core {
	fn from(value: Cid) -> Self {
		Core::Wasm(value)
	}
}
