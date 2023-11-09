use crate::{
	co_v1::{c_size_t, c_ssize_t},
	library::result_size,
	Cid,
};
use std::ffi::c_void;

pub fn write_cid(f: unsafe extern "C" fn(buffer: *const c_void, buffer_size: c_size_t) -> c_ssize_t, cid: &Cid) {
	let cid_bytes = cid.to_bytes();
	let size = result_size(unsafe { f(cid_bytes.as_ptr() as *const c_void, cid_bytes.len()) });
	assert_eq!(cid_bytes.len(), size);
}
