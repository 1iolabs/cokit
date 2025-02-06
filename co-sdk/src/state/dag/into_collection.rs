use co_primitives::DagCollectionAsyncExt;
use co_storage::{BlockStorage, StorageError};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;

/// Load all items of an [`NodeContainer`] into an collection.
///
/// See:
/// - [`co_api::DagCollection`]
/// - [`Vec`]
/// - [`std::collections::BTreeSet`]
/// - [`std::collections::BTreeMap`]
pub async fn into_collection<C, T, N, S>(storage: &S, container: &N) -> Result<C, StorageError>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: DagCollectionAsyncExt<Item = T>,
	C: Default + Extend<T>,
{
	container.stream(storage).try_collect().await
}
