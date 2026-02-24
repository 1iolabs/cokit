// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{EntryBlock, LogError};
use cid::Cid;
use co_primitives::BlockStorage;
use futures::{stream, StreamExt, TryStreamExt};

pub async fn get_entry_block<S>(storage: &S, cid: &Cid) -> Result<EntryBlock, LogError>
where
	S: BlockStorage + Send + Sync,
{
	let block = storage.get(cid).await?;
	Ok(EntryBlock::from_block(block)?)
}

pub async fn get_entry_blocks<S>(storage: &S, cids: impl Iterator<Item = &Cid>) -> Result<Vec<EntryBlock>, LogError>
where
	S: BlockStorage + Send + Sync,
{
	// get_entry_block_stream(storage, cids).try_collect().await

	// Ok(Default::default())

	stream::iter(cids)
		.then(move |cid| async move { get_entry_block(storage, cid).await })
		.try_collect()
		.await

	// stream::iter(cids)
	// 	.then(|cid| get_entry_block(storage, &cid))
	// 	.try_collect()
	// 	.await
}

// pub fn get_entry_block_stream<'s: 'i, 'i, S>(
// 	storage: &'s S,
// 	cids: impl Iterator<Item = &'i Cid> + 'i,
// ) -> impl Stream<Item = Result<EntryBlock<S::StoreParams>, LogError>> + 's
// where
// 	S: BlockStorage + Send + Sync,
// {
// 	stream::iter(cids).then(move |cid| async move { get_entry_block(storage, cid).await })
// }
