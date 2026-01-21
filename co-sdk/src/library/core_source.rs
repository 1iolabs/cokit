use crate::Cores;
use anyhow::anyhow;
use cid::Cid;
use co_primitives::{unixfs_add, AnyBlockStorage, Tags};
use futures::io::Cursor;
use serde::Serialize;

/// This module provides an enumeration for core sources, which can be references,
/// built-in cores identified by name, or byte representations of cores.
///
/// The CoreSource enum and its methods facilitate retrieving and managing
/// core binaries from various sources, including storage, predefined
/// built-ins, or raw bytes. The primary use case is to obtain a binary representation
/// of a core which can then be used to create a Core or Guard object for execution.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CoreSource {
	Reference(Cid),
	Builtin(String),
	Bytes(Vec<u8>),
}
impl CoreSource {
	pub fn built_in(name: impl Into<String>) -> Self {
		Self::Builtin(name.into())
	}

	pub async fn to_core(
		&self,
		storage: &impl AnyBlockStorage,
		cores: &Cores,
		tags: Tags,
	) -> Result<co_core_co::Core, anyhow::Error> {
		Ok(co_core_co::Core { binary: self.binary(storage, cores).await?, state: None, tags })
	}

	pub async fn to_core_create(
		&self,
		storage: &impl AnyBlockStorage,
		cores: &Cores,
		core: impl Into<String>,
		tags: Tags,
	) -> Result<co_core_co::CoAction, anyhow::Error> {
		Ok(co_core_co::CoAction::CoreCreate { core: core.into(), binary: self.binary(storage, cores).await?, tags })
	}

	pub async fn to_guard(
		&self,
		storage: &impl AnyBlockStorage,
		cores: &Cores,
		tags: Tags,
	) -> Result<co_core_co::Guard, anyhow::Error> {
		Ok(co_core_co::Guard { binary: self.binary(storage, cores).await?, tags })
	}

	pub async fn binary(&self, storage: &impl AnyBlockStorage, cores: &Cores) -> Result<Cid, anyhow::Error> {
		Ok(match self {
			CoreSource::Reference(binary) => *binary,
			CoreSource::Bytes(binary_bytes) => store_binary(storage, binary_bytes).await?,
			CoreSource::Builtin(name) => {
				let (cid, core) = cores.built_in_by_name(name).ok_or(anyhow!("Unknown built-in core: {}", name))?;

				// store
				let binary_cid = match &core {
					co_runtime::Core::Binary(binary_bytes) => Some(store_binary(storage, binary_bytes).await?),
					co_runtime::Core::Wasm(binary_cid) => Some(*binary_cid),
					_ => None,
				};

				// verify
				//  this should never happen
				//  indicates a corrupt build (wrong Cores.toml)
				//  or different unixfs encoding
				if let Some(binary_cid) = binary_cid {
					if binary_cid != cid {
						return Err(anyhow!("Builtin Core CID diverges: {}: {} != {}", name, cid, binary_cid));
					}
				}

				// result
				cid
			},
		})
	}
}

async fn store_binary(storage: &impl AnyBlockStorage, binary_bytes: &[u8]) -> Result<Cid, anyhow::Error> {
	let mut binary_stream = Cursor::new(binary_bytes);
	let binary = unixfs_add(storage, &mut binary_stream)
		.await?
		.pop()
		.ok_or(anyhow!("Add Core binary failed {}", binary_bytes.len()))?;
	Ok(binary)
}
