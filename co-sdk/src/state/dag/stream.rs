use co_primitives::Streamable;
use co_storage::{BlockStorage, StorageError};
use futures::Stream;
use serde::de::DeserializeOwned;

/// Stream element of an [`NodeContainer`].
///
/// See: [`co_api::DagCollection`]
pub fn stream<T, N, S>(storage: S, container: &N) -> impl Stream<Item = Result<T, StorageError>> + 'static
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: Streamable<S, Item = Result<T, StorageError>>,
{
	container.stream(storage)
}
