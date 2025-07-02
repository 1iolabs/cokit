use crate::{unixfs_add, BlockStorage, StorageError};
use anyhow::{anyhow, Context};
use cid::Cid;
use std::path::Path;
use tokio_util::compat::TokioAsyncReadCompatExt;

/// Store file to storage and return the (root) CID.
pub async fn unixfs_add_file<'a, S>(storage: &S, file: impl AsRef<Path>) -> Result<Cid, anyhow::Error>
where
	S: BlockStorage + Send + Sync,
{
	let mut handle = tokio::fs::File::open(file.as_ref())
		.await
		.with_context(|| format!("open file: {:?}", file.as_ref()))?
		.compat();
	Ok(unixfs_add(storage, &mut handle)
		.await?
		.last()
		.ok_or(StorageError::InvalidArgument(anyhow!("No CID generated: {:?}", file.as_ref())))? // we should have at least an empty block
		.to_owned())
}
