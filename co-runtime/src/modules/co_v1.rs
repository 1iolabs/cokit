// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::RuntimeContext;
use co_api::{Block, Cid, DefaultParams};
use co_storage::{Storage, StorageError};
#[cfg(not(feature = "js"))]
use std::time::Duration;
use std::{cmp::min, fmt::Debug, mem::swap};

//#[cfg(not(feature = "js"))]
pub type CoV1ApiStorageBox = Box<dyn Storage<StoreParams = DefaultParams> + Send + Sync>;
// #[cfg(feature = "js")]
// pub type CoV1ApiStorageBox = Box<dyn Storage<StoreParams = DefaultParams>>;

pub struct CoV1Api {
	storage: CoV1ApiStorageBox,
	context: RuntimeContext,
}
impl CoV1Api {
	pub fn new(storage: CoV1ApiStorageBox, context: RuntimeContext) -> Self {
		Self { storage, context }
	}

	pub fn state(&self) -> &Option<Cid> {
		&self.context.state
	}

	pub fn context(&self) -> &RuntimeContext {
		&self.context
	}

	pub fn set_state(&mut self, state: Cid) {
		self.context.state = Some(state);
	}

	pub fn context_mut(&mut self) -> &mut RuntimeContext {
		&mut self.context
	}

	pub fn swap(&mut self, other: &mut CoV1Api) {
		swap(&mut self.storage, &mut other.storage);
		swap(&mut self.context, &mut other.context);
	}

	pub fn storage(&mut self) -> &dyn Storage<StoreParams = DefaultParams> {
		self.storage.as_ref()
	}

	pub fn storage_mut(&mut self) -> &mut dyn Storage<StoreParams = DefaultParams> {
		self.storage.as_mut()
	}

	pub fn into_inner(self) -> (CoV1ApiStorageBox, RuntimeContext) {
		(self.storage, self.context)
	}

	/// Whether is error is retriable with same parameters.
	#[cfg(not(feature = "js"))]
	fn is_retriable(error: &StorageError) -> bool {
		matches!(error, StorageError::NotFound(_, _))
	}
}
impl Debug for CoV1Api {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoV1Api")
			// .field("storage", &"storage")
			.field("context", &self.context)
			.finish()
	}
}
impl Storage for CoV1Api {
	type StoreParams = DefaultParams;

	/// Get block in deterministic fashion.
	/// Note: If this function fails it will trap the core.
	/// Todo: Implement diagnostics.
	fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		#[cfg(feature = "js")]
		{
			self.storage.get(cid)
		}
		#[cfg(not(feature = "js"))]
		{
			let mut tries = 0;
			loop {
				return match self.storage.get(cid) {
					Ok(b) => Ok(b),
					Err(e) if Self::is_retriable(&e) && tries < 10 => {
						tries += 1;

						// log
						tracing::warn!(?cid, tries, "runtime-get-block-retry");

						// wait with exponential backoff
						std::thread::sleep(Duration::from_millis(2u64.pow(tries) * 1000));

						// retry
						continue;
					},
					Err(e) => Err(e),
				};
			}
		}
	}

	/// Set block in deterministic fashion.
	/// Note: If this function fails it will trap the core.
	/// Todo: Implement diagnostics.
	/// Todo: implement retries etc.
	fn set(&mut self, block: Block) -> Result<Cid, StorageError> {
		self.storage.set(block.with_store_params::<Self::StoreParams>()?)
	}

	fn remove(&mut self, _cid: &Cid) -> Result<(), StorageError> {
		unimplemented!()
	}
}

pub fn storage_block_get(api: &mut CoV1Api, cid: &[u8], buffer: &mut [u8]) -> Result<u32, anyhow::Error> {
	let cid = Cid::try_from(cid)?;
	let block = api.get(&cid)?;
	let size = min(block.data().len(), buffer.len());
	buffer[0..size].copy_from_slice(&block.data()[0..size]);
	Ok(block.data().len().try_into()?)
}

pub fn storage_block_set(api: &mut CoV1Api, cid: &[u8], buffer: &[u8]) -> Result<u32, anyhow::Error> {
	let cid = Cid::try_from(cid)?;
	let block = Block::new_unchecked(cid, Vec::from(buffer));
	let result = block.data().len().try_into()?;
	api.set(block)?;
	Ok(result)
}
