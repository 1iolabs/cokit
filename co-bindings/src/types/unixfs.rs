// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
