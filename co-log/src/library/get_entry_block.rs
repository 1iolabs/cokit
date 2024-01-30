use crate::{EntryBlock, LogError};
use co_storage::BlockStorage;
use futures::{stream, Stream, StreamExt, TryStreamExt};
use libipld::Cid;

pub async fn get_entry_block<'a, S>(storage: &'a S, cid: &'a Cid) -> Result<EntryBlock<S::StoreParams>, LogError>
where
	S: BlockStorage + Send + Sync + 'a,
{
	let block = storage.get(cid).await?;
	Ok(EntryBlock::from_block(block)?)
}

pub async fn get_entry_blocks<'a, S>(
	storage: &'a S,
	cids: impl Iterator<Item = &'a Cid> + 'a,
) -> Result<Vec<EntryBlock<S::StoreParams>>, LogError>
where
	S: BlockStorage + Send + Sync + 'a,
{
	get_entry_block_stream(storage, cids).try_collect().await
}

pub fn get_entry_block_stream<'a, S>(
	storage: &'a S,
	cids: impl Iterator<Item = &'a Cid> + 'a,
) -> impl Stream<Item = Result<EntryBlock<S::StoreParams>, LogError>> + 'a
where
	S: BlockStorage + Send + Sync + 'a,
{
	stream::iter(cids).then(|cid| get_entry_block(storage, cid))
}
