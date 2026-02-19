// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use std::{
	io::{self, ErrorKind},
	path::Path,
};
use tokio::fs;

pub async fn fs_write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>, create_dir_all: bool) -> io::Result<()> {
	match fs::write(path.as_ref(), contents.as_ref()).await {
		Err(e) if create_dir_all && e.kind() == ErrorKind::NotFound => {
			// create parent dir
			fs::create_dir_all(path.as_ref().parent().ok_or::<io::Error>(ErrorKind::NotFound.into())?).await?;

			// retry write
			fs::write(path, contents).await
		},
		i => i,
	}
}
