// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::Cid;

pub fn write_cid(f: unsafe extern "C" fn(buffer: *const u8, buffer_size: u32) -> u32, cid: &Cid) {
	let cid_bytes = cid.to_bytes();
	let size = unsafe { f(cid_bytes.as_ptr(), cid_bytes.len().try_into().expect("u32")) };
	assert_eq!(cid_bytes.len(), size as usize);
}
