// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::CoDate;
use co_identity::PrivateIdentity;
use co_primitives::{BlockSerializer, Did, Link, ReducerAction};
use co_storage::{ExtendedBlockOptions, ExtendedBlockStorage, StorageError};
use ipld_core::{ipld::Ipld, serde::to_ipld};
use serde::Serialize;

/// New reducer action.
pub fn new_reducer_action(
	from: impl Into<Did>,
	core: impl Into<String>,
	payload: impl Serialize,
	time: &impl CoDate,
) -> Result<ReducerAction<Ipld>, anyhow::Error> {
	Ok(ReducerAction { core: core.into(), from: from.into(), time: time.now(), payload: to_ipld(payload)? })
}

/// New reducer action.
pub fn new_typed_reducer_action<T>(
	from: impl Into<Did>,
	core: impl Into<String>,
	payload: T,
	time: &impl CoDate,
) -> ReducerAction<T> {
	ReducerAction { core: core.into(), from: from.into(), time: time.now(), payload }
}

/// Create and store reducer action.
pub async fn create_reducer_action<P, I, S>(
	storage: &S,
	from: &I,
	core: impl Into<String>,
	payload: P,
	options: ExtendedBlockOptions,
	time: &impl CoDate,
) -> Result<Link<ReducerAction<Ipld>>, StorageError>
where
	P: Serialize + Send + Sync,
	I: PrivateIdentity + Send + Sync,
	S: ExtendedBlockStorage,
{
	let action = new_typed_reducer_action(from.identity(), core, payload, time);
	store_reducer_action(storage, &action, options).await
}

/// Store reducer action.
pub async fn store_reducer_action<P, S>(
	storage: &S,
	action: &ReducerAction<P>,
	options: ExtendedBlockOptions,
) -> Result<Link<ReducerAction<Ipld>>, StorageError>
where
	P: Serialize + Send + Sync,
	S: ExtendedBlockStorage,
{
	let block = BlockSerializer::new().serialize(action)?;
	let cid = storage.set_extended((block, options).into()).await?;
	Ok(cid.into())
}
