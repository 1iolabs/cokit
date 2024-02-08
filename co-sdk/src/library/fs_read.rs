use std::{
	io::{self, ErrorKind},
	path::Path,
};
use tokio::fs;

/// Read file contents by returning None if file not exists.
pub async fn fs_read_option(path: impl AsRef<Path>) -> io::Result<Option<Vec<u8>>> {
	match fs::read(path).await {
		Ok(data) => Ok(Some(data)),
		Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
		Err(e) => Err(e),
	}
}
