// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
