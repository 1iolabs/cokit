// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_primitives::Streamable;
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
	N: Streamable<S, Item = Result<T, StorageError>>,
	C: Default + Extend<T>,
{
	container.stream(storage.clone()).try_collect().await
}
