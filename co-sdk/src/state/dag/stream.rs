// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_primitives::Streamable;
use co_storage::{BlockStorage, StorageError};
use futures::Stream;
use serde::de::DeserializeOwned;

/// Stream elements of a [`Streamable`] container.
pub fn stream<T, N, S>(storage: S, container: &N) -> impl Stream<Item = Result<T, StorageError>> + 'static
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: Streamable<S, Item = Result<T, StorageError>>,
{
	container.stream(storage)
}
