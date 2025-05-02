use crate::{reducer::core_resolver::dynamic::DynamicCoreResolver, CoStorage, CoreResolver, Reducer};
use async_trait::async_trait;
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
