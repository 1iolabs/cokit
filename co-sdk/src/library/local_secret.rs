use super::{fs_read::fs_read_option, fs_write::fs_write};
use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;
use std::{io::ErrorKind, path::PathBuf};

#[async_trait]
pub trait LocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error>;
}

pub struct MemoryLocalSecret {
	secret: co_storage::Secret,
}
impl MemoryLocalSecret {
	pub fn new() -> Self {
		Self { secret: Algorithm::default().generate_serect() }
	}
}
#[async_trait]
impl LocalSecret for MemoryLocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error> {
		Ok(self.secret.clone().into())
	}
}

pub struct KeychainLocalSecret {
	service: String,
	user: String,
}
impl KeychainLocalSecret {
	pub fn new(service: String, user: String) -> Self {
		Self { service, user }
	}

	/// Get or create encryption key in OS Keychain.
	fn fetch_secret_keychain(service: &str, user: &str, allow_create: bool) -> Result<Secret, anyhow::Error> {
		let entry = keyring::Entry::new(service, user)?;
		let key_as_base64 = match entry.get_password() {
			Ok(p) => p,
			Err(keyring::Error::NoEntry) if allow_create => {
				// generate and set key
				let secret = Algorithm::default().generate_serect();
				let secret_base64 = multibase::encode(multibase::Base::Base64, secret.divulge());
				entry.set_password(&secret_base64)?;

				// fetch again to make sure the key has persisted
				return Self::fetch_secret_keychain(service, user, false)
			},
			Err(e) => return Err(e.into()),
		};
		Ok(Secret::new(multibase::decode(key_as_base64)?.1))
	}
}
#[async_trait]
impl LocalSecret for KeychainLocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error> {
		Self::fetch_secret_keychain(&self.service, &self.user, true)
	}
}

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
