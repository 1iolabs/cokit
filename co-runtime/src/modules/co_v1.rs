use crate::RuntimeContext;
use co_api::{Block, Cid, Storage as ApiStorage};
use co_storage::{Storage, StorageError};
use libipld::DefaultParams;
use std::{cmp::min, fmt::Debug, mem::swap, time::Duration};

pub struct CoV1Api {
	storage: Box<dyn Storage<StoreParams = DefaultParams> + Send + Sync>,
	context: RuntimeContext,
}
impl CoV1Api {
	pub fn new(storage: Box<dyn Storage<StoreParams = DefaultParams> + Send + Sync>, context: RuntimeContext) -> Self {
		Self { storage, context }
	}

	pub fn state(&self) -> &Option<Cid> {
		&self.context.state
	}

	pub fn set_state(&mut self, state: Cid) {
		self.context.state = Some(state);
	}

	pub fn event(&self) -> &Cid {
		&self.context.event
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

	pub fn into_inner(self) -> (Box<dyn Storage<StoreParams = DefaultParams> + Send + Sync>, RuntimeContext) {
		(self.storage, self.context)
	}

	/// Whether is error is retriable with same parameters.
	fn is_retriable(error: &StorageError) -> bool {
		match error {
			StorageError::NotFound(_, _) => true,
			_ => false,
		}
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
impl ApiStorage for CoV1Api {
	/// Get block in deterministic fashion.
	/// Todo: Implement diagnostics.
	fn get(&self, cid: &libipld::Cid) -> co_api::Block {
		let mut tries = 0;
		loop {
			return match self.storage.get(cid) {
				Ok(b) => b,
				Err(e) if Self::is_retriable(&e) && tries < 10 => {
					tries += 1;

					// wait with exponential backoff
					std::thread::sleep(Duration::from_millis(2u64.pow(tries) * 1000));

					// retry
					continue;
				},
				Err(e) => Err(e).expect("get storage"),
			}
		}
	}

	/// Set block in deterministic fashion.
	/// Todo: Implement diagnostics.
	/// Todo: implement retries etc.
	fn set(&mut self, block: co_api::Block) -> Cid {
		self.storage.set(block).expect("set storage")
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CoV1ApiError {
	#[error("Invalid argument supplied from WASM")]
	InvalidArgument,
}
impl From<libipld::cid::Error> for CoV1ApiError {
	fn from(value: libipld::cid::Error) -> Self {
		match value {
			libipld::cid::Error::UnknownCodec => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::InputTooShort => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::ParsingError => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::InvalidCidVersion => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::InvalidCidV0Codec => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::InvalidCidV0Multihash => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::InvalidCidV0Base => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::VarIntDecodeError => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::Io(_) => CoV1ApiError::InvalidArgument,
			libipld::cid::Error::InvalidExplicitCidV0 => CoV1ApiError::InvalidArgument,
		}
	}
}

pub fn storage_block_get(api: &mut CoV1Api, cid: &[u8], buffer: &mut [u8]) -> Result<u32, CoV1ApiError> {
	// let cid_buffer: &[u8] = unsafe { from_raw_parts(cid as *const u8, cid_size) };
	let cid = Cid::try_from(cid)?;
	let block = api.get(&cid);
	let size = min(block.data().len(), buffer.len());
	buffer[0..size].copy_from_slice(&block.data()[0..size]);
	// unsafe { copy_nonoverlapping(block.data().as_ptr(), buffer as *mut u8, min(block.data().len(), buffer_size)) };
	Ok(block.data().len().try_into().expect("u32"))
}

pub fn storage_block_set(api: &mut CoV1Api, cid: &[u8], buffer: &[u8]) -> Result<u32, CoV1ApiError> {
	let cid = Cid::try_from(cid)?;
	let block = Block::new_unchecked(cid, Vec::from(buffer));
	let result = block.data().len().try_into().expect("u32");
	api.set(block);
	Ok(result)
}

pub fn state_cid_read(api: &CoV1Api, buffer: &mut [u8]) -> u32 {
	match api.context.state {
		Some(cid) => {
			let cid_buffer = cid.to_bytes();
			let size = min(buffer.len(), cid_buffer.len());
			buffer[0..size].copy_from_slice(&cid_buffer.as_slice()[0..size]);
			cid_buffer.len().try_into().expect("u32")
		},
		None => 0,
	}
}

pub fn state_cid_write(api: &mut CoV1Api, buffer: &[u8]) -> Result<u32, CoV1ApiError> {
	api.context.state = Some(Cid::try_from(buffer)?);
	Ok(buffer.len().try_into().expect("u32"))
}

pub fn event_cid_read(api: &CoV1Api, buffer: &mut [u8]) -> u32 {
	let cid_buffer = api.context.event.to_bytes();
	let size = min(buffer.len(), cid_buffer.len());
	buffer[0..size].copy_from_slice(&cid_buffer.as_slice()[0..size]);
	cid_buffer.len().try_into().expect("u32")
}
