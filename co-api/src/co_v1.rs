#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "co_v1")]
extern "C" {
	/// Read block.
	///
	/// Returns the byte length of the block.
	/// If the buffer_size is smaller than the returned byte length only the the first bytes until buffer_size are
	/// placed in buffer. The caller may call this again with an larger buffer.
	/// Also it is possible to call it with buffer_size=0 to only retrieve the size of the block.
	pub fn storage_block_get(cid: *const u8, cid_size: u32, buffer: *mut u8, buffer_size: u32) -> u32;

	/// Write block.
	pub fn storage_block_set(cid: *const u8, cid_size: u32, buffer: *const u8, buffer_size: u32) -> u32;

	/// Read binary runtime payload.
	/// Returns the total byte length of the payload.
	pub fn payload_read(buffer: *mut u8, buffer_size: u32, offset: u32) -> u32;

	/// Read state CID.
	/// Returns the byte length of the cid.
	/// If the buffer_size is smaller than the returned byte length only the the first bytes until buffer_size are
	/// placed in buffer. The caller may call this again with an larger buffer.
	/// If no prior state exists the result is zero.
	/// Encoding: Binary
	pub fn state_cid_read(buffer: *mut u8, buffer_size: u32) -> u32;

	/// Write state CID.
	/// Encoding: Binary
	pub fn state_cid_write(buffer: *const u8, buffer_size: u32) -> u32;

	/// Read event CID.
	/// Returns the byte length of the cid.
	/// If the buffer_size is smaller than the returned byte length only the the first bytes until buffer_size are
	/// placed in buffer. The caller may call this again with an larger buffer.
	/// If no prior state exists the result is zero.
	/// Encoding: Binary
	pub fn event_cid_read(buffer: *mut u8, buffer_size: u32) -> u32;

	/// Write diagnostic CID.
	/// Encoding: Binary
	pub fn diagnostic_cid_write(buffer: *const u8, buffer_size: u32) -> u32;
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn storage_block_get(
	_cid: *const u8,
	_cid_size: u32,
	_buffer: *mut u8,
	_buffer_size: u32,
) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn storage_block_set(
	_cid: *const u8,
	_cid_size: u32,
	_buffer: *const u8,
	_buffer_size: u32,
) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn payload_read(_buffer: *mut u8, _buffer_size: u32, _offset: u32) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn state_cid_read(_buffer: *mut u8, _buffer_size: u32) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn state_cid_write(_buffer: *const u8, _buffer_size: u32) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn event_cid_read(_buffer: *mut u8, _buffer_size: u32) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}

/// Stub
///
/// # Safety
/// This is only a stub to prevent mistakes.
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn diagnostic_cid_write(_buffer: *const u8, _buffer_size: u32) -> u32 {
	panic!("only available for target_arch = \"wasm32\"");
}
