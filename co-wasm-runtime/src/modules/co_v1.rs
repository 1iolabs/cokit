use co_storage::{Storage, StorageError};
use co_wasm_api::{Block, Cid};
use std::{cmp::min, fmt::Debug};

pub struct CoV1Api {
	storage: Box<dyn Storage + Send + Sync>,
	state: Option<Cid>,
	event: Cid,
}
impl CoV1Api {
	pub fn new(storage: Box<dyn Storage + Send + Sync>, state: Option<Cid>, event: Cid) -> Self {
		Self { storage, state, event }
	}

	pub fn state(&self) -> &Option<Cid> {
		&self.state
	}
}
impl Debug for CoV1Api {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CoV1Api")
			// .field("storage", &"storage")
			.field("state", &self.state)
			.field("event", &self.event)
			.finish()
	}
}

#[derive(Debug, thiserror::Error)]
pub enum CoV1ApiError {
	#[error("Invalid argument supplied from WASM")]
	InvalidArgument,

	#[error("Storage error")]
	Storage(#[from] StorageError),
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
impl CoV1ApiError {
	/// Whether is error is retriable with same parameters.
	pub fn is_retriable(&self) -> bool {
		match self {
			CoV1ApiError::Storage(StorageError::NotFound) => true,
			_ => false,
		}
	}
}

pub fn storage_block_get(api: &mut CoV1Api, cid: &[u8], buffer: &mut [u8]) -> Result<u32, CoV1ApiError> {
	// let cid_buffer: &[u8] = unsafe { from_raw_parts(cid as *const u8, cid_size) };
	let cid = Cid::try_from(cid)?;
	let block = api.storage.get(&cid)?;
	let size = min(block.data().len(), buffer.len());
	buffer[0..size].copy_from_slice(&block.data()[0..size]);
	// unsafe { copy_nonoverlapping(block.data().as_ptr(), buffer as *mut u8, min(block.data().len(), buffer_size)) };
	Ok(block.data().len().try_into().expect("u32"))
}

pub fn storage_block_set(api: &mut CoV1Api, cid: &[u8], buffer: &[u8]) -> Result<u32, CoV1ApiError> {
	let cid = Cid::try_from(cid)?;
	let block = Block::new_unchecked(cid, Vec::from(buffer));
	let result = block.data().len().try_into().expect("u32");
	api.storage.set(block)?;
	Ok(result)
}

pub fn state_cid_read(api: &CoV1Api, buffer: &mut [u8]) -> u32 {
	match api.state {
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
	api.state = Some(Cid::try_from(buffer)?);
	Ok(buffer.len().try_into().expect("u32"))
}

pub fn event_cid_read(api: &CoV1Api, buffer: &mut [u8]) -> u32 {
	let cid_buffer = api.event.to_bytes();
	let size = min(buffer.len(), cid_buffer.len());
	buffer[0..size].copy_from_slice(&cid_buffer.as_slice()[0..size]);
	cid_buffer.len().try_into().expect("u32")
}
