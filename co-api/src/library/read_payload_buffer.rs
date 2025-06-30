use crate::co_v1::payload_read;
use std::cmp::min;

/// Read full payload into a buffer.
pub fn read_payload_buffer() -> Vec<u8> {
	let mut result = Vec::new();
	let mut buffer = [0u8; 1024];
	let mut offset: usize = 0;
	loop {
		let total = payload_read(buffer.as_mut_ptr(), buffer.len() as u32, offset.try_into().expect("u32")) as usize;
		let read = min(total - offset, buffer.len());

		// offset
		offset += read;

		// copy
		result.extend_from_slice(&buffer[0..read]);

		// done?
		if !(result.len() < total) {
			break;
		}
	}
	result
}
