use crate::Cid;

pub fn write_cid(f: unsafe extern "C" fn(buffer: *const u8, buffer_size: u32) -> u32, cid: &Cid) {
	let cid_bytes = cid.to_bytes();
	let size = unsafe { f(cid_bytes.as_ptr(), cid_bytes.len().try_into().expect("u32")) };
	assert_eq!(cid_bytes.len(), size as usize);
}
