// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use std::{
	io::{self, ErrorKind},
	path::Path,
};

/// Read file contents by returning None if file not exists.
pub async fn fs_read_option(path: impl AsRef<Path>) -> io::Result<Option<Vec<u8>>> {
	match tokio::fs::read(path).await {
		Ok(data) => Ok(Some(data)),
		Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
		Err(e) => Err(e),
	}
}
