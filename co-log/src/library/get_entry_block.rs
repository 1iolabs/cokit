use crate::{EntryBlock, LogError};
use co_storage::BlockStorage;
use futures::{stream, StreamExt, TryStreamExt};
use libipld::Cid;

pub async fn get_entry_block<S>(storage: &S, cid: &Cid) -> Result<EntryBlock<S::StoreParams>, LogError>
where
	S: BlockStorage + Send + Sync,
{
	let block = storage.get(cid).await?;
	Ok(EntryBlock::from_block(block)?)
}

pub async fn get_entry_blocks<S>(
	storage: &S,
	cids: impl Iterator<Item = &Cid>,
) -> Result<Vec<EntryBlock<S::StoreParams>>, LogError>
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
