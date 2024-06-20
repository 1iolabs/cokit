use crate::Cid;

pub fn read_cid(f: unsafe extern "C" fn(buffer: *mut u8, buffer_size: u32) -> u32) -> Option<Cid> {
	let mut buffer: [u8; 256] = [0; 256];
	let size = unsafe { f(buffer.as_mut_ptr(), 256) };
	match size {
		0 => None,
		_ if size > 256 => {
			let mut buffer = vec![0u8; size as usize];
			let size = unsafe { f(buffer.as_mut_ptr(), size) };
			assert_eq!(buffer.len(), size as usize);
			Some(Cid::try_from(&buffer[0..size as usize]).expect("valid CID"))
		},
		_ => Some(Cid::try_from(&buffer[0..size as usize]).expect("valid CID")),
	}
}
