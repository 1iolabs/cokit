use crate::state::stream;
use co_primitives::NodeContainer;
use co_storage::{BlockStorage, StorageError};
use futures::StreamExt;
use serde::de::DeserializeOwned;

/// Returns `true` if the [`NodeContainer`] collection contains no elements.
pub async fn is_empty<T, N, S>(storage: &S, container: &N) -> Result<bool, StorageError>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: NodeContainer<T>,
{
	// its always empty if we have no link
	if container.node_container_link().is_none() {
		return Ok(true);
	}

	// to be sure check if we have an empty root node
	match stream(storage.clone(), container).next().await {
		None => Ok(true),
		Some(Err(e)) => Err(e),
		Some(_) => Ok(false),
	}
}
