use co_identity::PrivateIdentity;
use co_primitives::{Did, Link, ReducerAction};
use co_storage::{BlockStorage, BlockStorageExt, StorageError};
use ipld_core::{ipld::Ipld, serde::to_ipld};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// New reducer action.
pub fn new_reducer_action(
	from: impl Into<Did>,
	core: impl Into<String>,
	payload: impl Serialize,
) -> Result<ReducerAction<Ipld>, anyhow::Error> {
	Ok(ReducerAction {
		core: core.into(),
		from: from.into(),
		time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis(),
		payload: to_ipld(payload)?,
	})
}

/// Create and store reducer action.
pub async fn create_reducer_action<P, I, S>(
	storage: &S,
	from: &I,
	core: impl Into<String>,
	payload: P,
) -> Result<Link<ReducerAction<Ipld>>, StorageError>
where
	P: Serialize + Send + Sync,
	I: PrivateIdentity + Send + Sync,
	S: BlockStorage,
{
	let action = ReducerAction {
		core: core.into(),
		payload,
		from: from.identity().to_owned(),
		time: SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis(),
	};
	store_reducer_action(storage, &action).await
}

/// Store reducer action.
pub async fn store_reducer_action<P, S>(
	storage: &S,
	action: &ReducerAction<P>,
) -> Result<Link<ReducerAction<Ipld>>, StorageError>
where
	P: Serialize + Send + Sync,
	S: BlockStorage,
{
	let cid = storage.set_serialized(&action).await?;
	Ok(cid.into())
}
