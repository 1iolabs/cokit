// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{co_v1, Block, BlockStorage, Cid, DefaultParams, Storage, StorageError};
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::{BlockStorageStoreParams, StoreParams};

/// Storage implementation for the co_v1 API.
pub struct WasmStorage {}
impl Default for WasmStorage {
	fn default() -> Self {
		Self::new()
	}
}
impl WasmStorage {
	pub fn new() -> Self {
		Self {}
	}
}
impl Storage for WasmStorage {
	fn get(&self, cid: &Cid) -> Block {
		wasm_block_get(cid)
	}

	fn set(&mut self, block: Block) -> Cid {
		wasm_block_set(block)
	}
}
#[async_trait]
impl BlockStorage for WasmStorage {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		Ok(wasm_block_get(cid))
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		Ok(wasm_block_set(block))
	}

	async fn remove(&self, _cid: &Cid) -> Result<(), StorageError> {
		Err(StorageError::Internal(anyhow!("Unsupported")))
	}

	fn max_block_size(&self) -> usize {
		<Self as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE
	}
}
impl BlockStorageStoreParams for WasmStorage {
	type StoreParams = DefaultParams;
}
impl Clone for WasmStorage {
	fn clone(&self) -> Self {
		Self {}
	}
}

fn wasm_block_get(cid: &Cid) -> Block {
	let cid_bytes = cid.to_bytes();

	// try to read block in 1KiB buffer
	let buffer_size = 2usize.pow(10); // 1024
	let mut buffer = vec![0; buffer_size];
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

fn wasm_block_set(block: Block) -> Cid {
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
