// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::StorageError;
use anyhow::{anyhow, Context};
use cid::Cid;
use co_primitives::{unixfs_add, AnyBlockStorage};
use std::path::Path;
use tokio_util::compat::TokioAsyncReadCompatExt;

/// Store file to storage and return the (root) CID.
pub async fn unixfs_add_file(storage: &impl AnyBlockStorage, file: impl AsRef<Path>) -> Result<Cid, anyhow::Error> {
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
