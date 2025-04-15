use crate::{co_v1, Block, BlockStorage, Cid, DefaultParams, Storage, StorageError};
use anyhow::anyhow;
use async_trait::async_trait;

/// Storage implementation for the co_v1 API.
pub struct WasmStorage {}
impl WasmStorage {
	pub fn new() -> Self {
		Self {}
	}
}
impl Storage for WasmStorage {
	fn get(&self, cid: &Cid) -> Block<DefaultParams> {
		wasm_block_get(cid)
	}

	fn set(&mut self, block: Block<DefaultParams>) -> Cid {
		wasm_block_set(block)
	}
}
#[async_trait]
impl BlockStorage for WasmStorage {
	type StoreParams = DefaultParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		Ok(wasm_block_get(cid))
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		Ok(wasm_block_set(block))
	}

	async fn remove(&self, _cid: &Cid) -> Result<(), StorageError> {
		Err(StorageError::Internal(anyhow!("Unsupported")))
	}
}
impl Clone for WasmStorage {
	fn clone(&self) -> Self {
		Self {}
	}
}

fn wasm_block_get(cid: &Cid) -> Block<DefaultParams> {
	let cid_bytes = cid.to_bytes();

	// try to read block in 1KiB buffer
	let buffer_size = 2usize.pow(10); // 1024
	let mut buffer = Vec::with_capacity(buffer_size);
	buffer.resize(buffer_size, 0);
	#[allow(unused_unsafe)]
	let block_size = unsafe {
		co_v1::storage_block_get(
			cid_bytes.as_ptr(),
			cid_bytes.len().try_into().expect("u32"),
			buffer.as_mut_ptr(),
			buffer.len().try_into().expect("u32"),
		)
	};

	// read again with larger buffer if block is larger
	if (block_size as usize) > buffer.len() {
		buffer.resize(block_size as usize, 0);
		#[allow(unused_unsafe)]
		let block_size = unsafe {
			co_v1::storage_block_get(
				cid_bytes.as_ptr(),
				cid_bytes.len().try_into().expect("u32"),
				buffer.as_mut_ptr(),
				buffer.len().try_into().expect("u32"),
			)
		};
		assert_eq!(buffer.len(), block_size as usize);
	}
	// truncate buffer to actual block size
	else if (block_size as usize) < buffer.len() {
		buffer.truncate(block_size as usize);
	}

	// result
	Block::new_unchecked(*cid, buffer)
}

fn wasm_block_set(block: Block<DefaultParams>) -> Cid {
	let cid_bytes = block.cid().to_bytes();
	#[allow(unused_unsafe)]
	let block_size = unsafe {
		co_v1::storage_block_set(
			cid_bytes.as_ptr(),
			cid_bytes.len().try_into().expect("u32"),
			block.data().as_ptr(),
			block.data().len().try_into().expect("u32"),
		)
	};
	assert_eq!(block.data().len(), block_size as usize);
	block.into_inner().0
}
