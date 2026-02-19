// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{BlockStorage, CoCid, CoError};
use anyhow::anyhow;
use co_sdk::unixfs_add;
use futures::io::Cursor;

/// Add bytes as unixfs to storage.
/// Returns the root CID of the unixfs.
pub async fn unixfs_add_buffer(storage: &BlockStorage, bytes: Vec<u8>) -> Result<CoCid, CoError> {
	let mut stream = Cursor::new(bytes);
	let cids = unixfs_add(storage, &mut stream).await.map_err(CoError::new)?;
	Ok(CoCid::from(cids.last().ok_or(CoError::new(anyhow!("Empty")))?))
}
