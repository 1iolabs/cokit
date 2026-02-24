// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::co_v1::payload_read;
use std::cmp::min;

/// Read full payload into a buffer.
pub fn read_payload_buffer() -> Vec<u8> {
	let mut result = Vec::new();
	let mut buffer = [0u8; 1024];
	let mut offset: usize = 0;
	loop {
		let total = {
			let read_buffer = buffer.as_mut_ptr();
			let read_buffer_len = buffer.len() as u32;
			let read_offset = offset.try_into().expect("u32");
			let read_total = unsafe { payload_read(read_buffer, read_buffer_len, read_offset) };
			read_total as usize
		};
		let read = min(total - offset, buffer.len());

		// offset
		offset += read;

		// copy
		result.extend_from_slice(&buffer[0..read]);

		// done?
		if result.len() >= total {
			break;
		}
	}
	result
}
