#![allow(non_camel_case_types)]

use std::ffi::c_void;

pub type c_ssize_t = isize;
pub type c_size_t = usize;

#[link(wasm_import_module = "co_v1")]
extern "C" {
	/// Read block.
	///
	/// Returns the byte length of the block.
	/// If the buffer_size is smaller than the returned byte length only the the first bytes until buffer_size are
	/// placed in buffer. The caller may call this again with an larger buffer.
	/// Also it is possible to call it with buffer_size=0 to only retrieve the size of the block.
	pub fn storage_block_get(
		cid: *const c_void,
		cid_size: c_size_t,
		buffer: *mut c_void,
		buffer_size: c_size_t,
	) -> c_ssize_t;

	/// Write block.
	pub fn storage_block_set(
		cid: *const c_void,
		cid_size: c_size_t,
		buffer: *const c_void,
		buffer_size: c_size_t,
	) -> c_ssize_t;

	/// Read state CID.
	/// Returns the byte length of the cid.
	/// If the buffer_size is smaller than the returned byte length only the the first bytes until buffer_size are
	/// placed in buffer. The caller may call this again with an larger buffer.
	/// Encoding: Binary
	pub fn state_cid_read(buffer: *mut c_void, buffer_size: c_size_t) -> c_ssize_t;

	/// Write state CID.
	/// Encoding: Binary
	pub fn state_cid_write(buffer: *const c_void, buffer_size: c_size_t) -> c_ssize_t;

	/// Read event CID.
	/// Returns the byte length of the cid.
	/// If the buffer_size is smaller than the returned byte length only the the first bytes until buffer_size are
	/// placed in buffer. The caller may call this again with an larger buffer.
	/// Encoding: Binary
	pub fn event_cid_read(buffer: *mut c_void, buffer_size: c_size_t) -> c_ssize_t;
}
