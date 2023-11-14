use crate::Cid;

pub fn read_cid(f: unsafe extern "C" fn(buffer: *mut u8, buffer_size: u32) -> u32) -> Cid {
	let mut buffer: [u8; 256] = [0; 256];
	let size = unsafe { f(buffer.as_mut_ptr(), 256) };
	if size > 256 {
		let mut buffer = Vec::<u8>::with_capacity(size as usize);
		buffer.resize(size as usize, 0);
		let size = unsafe { f(buffer.as_mut_ptr(), size) };
		assert_eq!(buffer.len(), size as usize);
		Cid::try_from(&buffer[0..size as usize]).expect("valid CID")
	} else {
		Cid::try_from(&buffer[0..size as usize]).expect("valid CID")
	}
}
