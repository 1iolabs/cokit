use crate::{reducer::core_resolver::dynamic::DynamicCoreResolver, CoStorage, CoreResolver, Reducer};
use async_trait::async_trait;
use co_primitives::Did;
use co_storage::{BlockStorageContentMapping, ExtendedBlockStorage};

#[async_trait]
pub trait ReducerFlush<S, R>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	async fn flush(&mut self, storage: &S, reducer: &Reducer<S, R>) -> anyhow::Result<()>;
}

pub type CoReducerFlush = Box<dyn ReducerFlush<CoStorage, DynamicCoreResolver<CoStorage>> + Send + Sync + 'static>;

#[derive(Debug, Default, Clone)]
pub struct FlushInfo {
	/// Flushed operations that has local origin.
	pub local: bool,

	/// The last identity that executed a local operation.
	/// Only set when local it true.
	pub local_identity: Option<Did>,

	/// Whether the co has a network feature.
	pub network: bool,
}
