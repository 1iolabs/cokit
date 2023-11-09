use crate::{co_v1, library::result_size, Block, Cid, Storage};
use std::ffi::c_void;

pub struct StorageApi {}

/// Storage implementation for the co_v1 API.
impl StorageApi {
	pub fn new() -> Self {
		Self {}
	}
}

impl Storage for StorageApi {
	fn get(&self, cid: &Cid) -> Block {
		let cid_bytes = cid.to_bytes();

		// try to read block in 1KiB buffer
		let buffer_size = 2 ^ 10; // 1024
		let mut buffer = Vec::with_capacity(buffer_size);
		buffer.resize(buffer_size, 0);
		let block_size = result_size(unsafe {
			co_v1::storage_block_get(
				cid_bytes.as_ptr() as *const c_void,
				cid_bytes.len(),
				buffer.as_mut_ptr() as *mut c_void,
				buffer.len(),
			)
		});

		// read again with larger buffer if block is larger
		if block_size > buffer.len() {
			buffer.resize(block_size as usize, 0);
			let block_size = result_size(unsafe {
				co_v1::storage_block_get(
					cid_bytes.as_ptr() as *const c_void,
					cid_bytes.len(),
					buffer.as_mut_ptr() as *mut c_void,
					buffer.len(),
				)
			});
			assert_eq!(buffer.len(), block_size);
		}
		// truncate buffer to actual block size
		else if block_size < buffer.len() {
			buffer.truncate(block_size);
		}

		// result
		Block::new_unchecked(cid.clone(), buffer)
	}

	fn set(&mut self, block: Block) {
		let cid_bytes = block.cid().to_bytes();
		let block_size = result_size(unsafe {
			co_v1::storage_block_set(
				cid_bytes.as_ptr() as *const c_void,
				cid_bytes.len(),
				block.data().as_ptr() as *mut c_void,
				block.data().len(),
			)
		});
		assert_eq!(block_size, block.data().len());
	}
}
