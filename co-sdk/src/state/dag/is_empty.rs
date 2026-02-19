// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::state::stream;
use co_primitives::Streamable;
use co_storage::{BlockStorage, StorageError};
use futures::{pin_mut, StreamExt};
use serde::de::DeserializeOwned;

/// Returns `true` if the [`NodeContainer`] collection contains no elements.
pub async fn is_empty<T, N, S>(storage: &S, container: &N) -> Result<bool, StorageError>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: Streamable<S, Item = Result<T, StorageError>>,
{
	// to be sure check if we have an empty root node
	let stream = stream(storage.clone(), container);
	pin_mut!(stream);
	match stream.next().await {
		None => Ok(true),
		Some(Err(e)) => Err(e),
		Some(_) => Ok(false),
	}
}
