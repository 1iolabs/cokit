// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{fs_read::fs_read_option, fs_write::fs_write};
use crate::library::local_secret::LocalSecret;
use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;
use std::{io::ErrorKind, path::PathBuf};

pub struct FileLocalSecret {
	key_path: PathBuf,
}
impl FileLocalSecret {
	pub fn new(file: PathBuf) -> Self {
		Self { key_path: file }
	}

	async fn fetch_secret_cbor(key_path: &PathBuf, allow_create: bool) -> Result<Secret, anyhow::Error> {
		match fs_read_option(key_path).await {
			Ok(Some(data)) => {
				let result: Secret = serde_ipld_dagcbor::from_slice(&data)?;
				Ok(result)
			},
			Ok(None) if allow_create => {
				// create
				let secret: Secret = Algorithm::default().generate_serect().into();
				let contents = serde_ipld_dagcbor::to_vec(&secret)?;
				fs_write(key_path, contents, true).await?;

				// result
				Ok(secret)
			},
			Ok(None) => Err(Into::<std::io::Error>::into(ErrorKind::NotFound).into()),
			Err(e) => Err(e.into()),
		}
	}
}
#[async_trait]
impl LocalSecret for FileLocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error> {
		Self::fetch_secret_cbor(&self.key_path, true).await
	}
}
