use anyhow::Context;
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
	pub async fn load(&self) -> Result<(Option<Cid>, BTreeSet<Cid>), anyhow::Error> {
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

		// result
		Ok((state, heads))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplicationLocal {
	#[serde(rename = "v")]
	pub version: u8,
	#[serde(rename = "h")]
	pub heads: Vec<Cid>,
	#[serde(rename = "s")]
	pub state: Cid,
}
impl ApplicationLocal {
	pub fn new(heads: Vec<Cid>, state: Cid) -> Self {
		Self { heads, state, version: 1 }
	}

	async fn read(path: &PathBuf) -> anyhow::Result<Option<ApplicationLocal>> {
		match tokio::fs::read(path).await {
			Ok(data) => {
				let result: ApplicationLocal = serde_ipld_dagcbor::from_slice(&data)?;
				if result.version != 1 {
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
}
