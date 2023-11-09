use crate::{
	co_v1::{c_size_t, c_ssize_t},
	library::result_size,
	Cid,
};
use std::ffi::c_void;

pub fn read_cid(f: unsafe extern "C" fn(buffer: *mut c_void, buffer_size: c_size_t) -> c_ssize_t) -> Cid {
	let mut buffer: [u8; 256] = [0; 256];
	let size = result_size(unsafe { f(buffer.as_mut_ptr() as *mut c_void, 256) });
	if size > 256 {
		let mut buffer = Vec::<u8>::with_capacity(size);
		buffer.resize(size, 0);
		let size = result_size(unsafe { f(buffer.as_mut_ptr() as *mut c_void, size) });
		assert_eq!(buffer.len(), size);
		Cid::try_from(&buffer[0..size]).expect("valid CID")
	} else {
		Cid::try_from(&buffer[0..size]).expect("valid CID")
	}
}
