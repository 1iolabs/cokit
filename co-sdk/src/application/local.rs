use crate::CoStorage;
use anyhow::Context;
use co_log::{IdentityResolver, LocalIdentityResolver, Log};
use co_storage::{Algorithm, BlockStorage, EncryptedBlockStorage, Secret};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io::ErrorKind, path::PathBuf};

pub struct LocalCo {
	/// Our application identifier.
	identifier: String,
	/// The application base path.
	application_path: PathBuf,
	/// Local CO State.
	state: Option<Cid>,
}
impl LocalCo {
	pub fn new(identifier: String, application_path: PathBuf) -> Self {
		Self { identifier, application_path, state: Default::default() }
	}

	/// Try to load the local co state from disk.
	pub async fn load<S>(&self, storage: S) -> Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		let mut heads: BTreeSet<Cid> = Default::default();
		let mut state: Option<Cid> = Default::default();

		// read applications
		let mut dir = tokio::fs::read_dir(&self.application_path).await?;
		while let Some(child) = dir.next_entry().await? {
			let local = ApplicationLocal::read(&child.path().join("local.cbor")).await?;
			if let Some(local) = local {
				// heads
				heads.extend(local.heads.iter());

				// state
				if child.file_name().as_encoded_bytes() == self.identifier.as_bytes() || state.is_none() {
					state = Some(local.state);
				}
			}
		}

		// TODO: state
		todo!();

		// result
		Ok((state, heads))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplicationLocal {
	/// The application local version.
	#[serde(rename = "v")]
	pub version: u8,

	/// The latest heads.
	/// Todo: Do we need this as this is encoded in the state anyway?
	#[serde(rename = "h")]
	pub heads: Vec<Cid>,

	/// The latest state.
	#[serde(rename = "s")]
	pub state: Cid,

	/// The latest encryption mapping.
	#[serde(rename = "m")]
	pub mapping: Cid,
}
impl ApplicationLocal {
	pub fn version() -> u8 {
		1
	}

	pub fn new(heads: Vec<Cid>, state: Cid, mapping: Cid) -> Self {
		Self { heads, state, version: Self::version(), mapping }
	}

	async fn read(path: &PathBuf) -> anyhow::Result<Option<ApplicationLocal>> {
		match tokio::fs::read(path).await {
			Ok(data) => {
				let result: ApplicationLocal = serde_ipld_dagcbor::from_slice(&data)?;
				if result.version != Self::version() {
					return Err(anyhow::anyhow!("Invalid file version"));
				}
				Ok(Some(result))
			},
			Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
			Err(e) => Err(e),
		}
		.context(format!("while reading file {:?}", path))
	}

	async fn write(&self, path: &PathBuf) -> anyhow::Result<()> {
		let data = serde_ipld_dagcbor::to_vec(self)?;
		tokio::fs::write(path, data).await?;
		Ok(())
	}

	pub async fn log<S>(&self, storage: S) -> Result<Log<EncryptedBlockStorage<S>>, anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		// encryption
		let entry = keyring::Entry::new_with_target("local", "co", "device")?;
		let key_as_base64 = match entry.get_password() {
			Ok(p) => p,
			Err(keyring::Error::NoEntry) => {
				let secret = Algorithm::default().generate_serect();
				multibase::encode(multibase::Base::Base64, secret.divulge())
			},
			Err(e) => return Err(e.into()),
		};
		let key = Secret::new(multibase::decode(key_as_base64)?.1);
		let mut encrypted_storage = EncryptedBlockStorage::new(storage.clone(), key);
		encrypted_storage.load_mapping(&self.mapping).await?;

		// log
		Ok(Log::new(
			"local".as_bytes().to_vec(),
			LocalIdentityResolver::default().private_identity("did:local:device")?,
			Box::new(LocalIdentityResolver::default()),
			encrypted_storage.clone(),
			self.heads.iter().cloned().collect(),
		))
	}
}
