// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_primitives::Streamable;
use co_storage::{BlockStorage, StorageError};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;

/// Load all items of a [`Streamable`] container into a collection.
pub async fn into_collection<C, T, N, S>(storage: &S, container: &N) -> Result<C, StorageError>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
	T: DeserializeOwned + Send + Sync + 'static,
	N: Streamable<S, Item = Result<T, StorageError>>,
	C: Default + Extend<T>,
{
	container.stream(storage.clone()).try_collect().await
}
