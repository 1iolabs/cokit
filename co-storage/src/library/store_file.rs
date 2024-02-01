use crate::{unixfs_add, BlockStorage};
use libipld::Cid;
use std::path::Path;
use tokio_util::compat::TokioAsyncReadCompatExt;

pub async fn store_file<'a, S>(storage: &'a S, file: impl AsRef<Path>) -> Result<Cid, std::io::Error>
where
	S: BlockStorage + Send + Sync + 'a,
{
	let mut handle = tokio::fs::File::open(file).await.unwrap().compat();
	Ok(unixfs_add(storage, &mut handle).await.unwrap().last().unwrap().to_owned())
}
