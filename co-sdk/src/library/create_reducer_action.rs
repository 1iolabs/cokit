use co_identity::PrivateIdentity;
use co_primitives::{BlockSerializer, Date, Did, Link, ReducerAction};
use co_storage::{ExtendedBlockOptions, ExtendedBlockStorage, StorageError};
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
	options: ExtendedBlockOptions,
	time: Option<Date>,
) -> Result<Link<ReducerAction<Ipld>>, StorageError>
where
	P: Serialize + Send + Sync,
	I: PrivateIdentity + Send + Sync,
	S: ExtendedBlockStorage,
{
	let action = ReducerAction {
		core: core.into(),
		payload,
		from: from.identity().to_owned(),
		time: time.unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).expect("Valid time").as_millis()),
	};
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
