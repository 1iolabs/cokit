// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

#[cfg(feature = "fs")]
use super::fs_read::fs_read_option;
use crate::CoReducerState;
#[cfg(feature = "fs")]
use anyhow::{anyhow, Context as _};
use async_trait::async_trait;
use cid::Cid;
#[cfg(feature = "fs")]
use co_primitives::from_cbor;
use futures::Stream;
use serde::{Deserialize, Serialize};
#[cfg(feature = "fs")]
use std::path::PathBuf;
use std::{collections::BTreeSet, fmt::Debug};

#[async_trait]
pub trait Locals: Clone + Debug + Send + Sync {
	/// Get current ApplicationLocal instances.
	async fn get(&self) -> Result<Vec<ApplicationLocal>, anyhow::Error>;

	/// Watch ApplicationLocal instances after last get.
	fn watch(&self) -> impl Stream<Item = ApplicationLocal> + Send + Sync + 'static;

	/// Set ApplicationLocal for our instance.
	async fn set(&mut self, local: ApplicationLocal) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationLocal {
	/// The application local version.
	#[serde(rename = "v")]
	pub version: u8,

	/// The latest heads.
	/// Todo: Do we need this as this is encoded in the state anyway?
	#[serde(rename = "h")]
	pub heads: BTreeSet<Cid>,

	/// The latest state.
	#[serde(rename = "s")]
	pub state: Cid,

	/// The latest encryption mapping.
	#[serde(rename = "m", skip_serializing_if = "Option::is_none", default)]
	pub mapping: Option<Cid>,
}
impl ApplicationLocal {
	pub fn version() -> u8 {
		1
	}

	pub fn new(heads: BTreeSet<Cid>, state: Cid, mapping: Option<Cid>) -> Self {
		Self { heads, state, version: Self::version(), mapping }
	}

	/// Read path as ApplicationLocal expecting DAG-CBOR format.
	/// Returns `None` if file not exists.
	#[cfg(feature = "fs")]
	#[tracing::instrument(level = tracing::Level::TRACE, name = "locals-read", err(Debug))]
	pub async fn read(path: &PathBuf) -> anyhow::Result<Option<ApplicationLocal>> {
		Ok(
			match fs_read_option(path)
				.await
				.with_context(|| format!("Reading file: {:?}", path))?
			{
				Some(data) => {
					let result: ApplicationLocal = from_cbor(&data)?;
					if result.version != Self::version() {
						return Err(anyhow!("Invalid file version"));
					}
					Some(result)
				},
				None => None,
			},
		)
	}

	pub fn reducer_state(&self) -> CoReducerState {
		(self.state, self.heads.clone()).into()
	}
}
