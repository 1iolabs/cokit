use crate::{unixfs_add, BlockStorage, StorageError};
use anyhow::anyhow;
use libipld::Cid;
use std::path::Path;
use tokio_util::compat::TokioAsyncReadCompatExt;

/// Store file to storage and return the (root) CID.
pub async fn unixfs_add_file<'a, S>(storage: &S, file: impl AsRef<Path>) -> Result<Cid, StorageError>
where
	S: BlockStorage + Send + Sync,
{
	let mut handle = tokio::fs::File::open(file.as_ref()).await.unwrap().compat();
	Ok(unixfs_add(storage, &mut handle)
		.await?
		.last()
		.ok_or(StorageError::InvalidArgument(anyhow!("File is empty: {:?}", file.as_ref())))?
		.to_owned())
}
