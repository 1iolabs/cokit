// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
