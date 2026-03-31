// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
