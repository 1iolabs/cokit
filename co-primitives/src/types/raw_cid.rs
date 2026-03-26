// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use std::io::Cursor;

pub const CID_MAX_SIZE: usize = 128;
pub type RawCid = [u8; CID_MAX_SIZE];

pub fn cid_to_raw(cid: &Cid) -> RawCid {
	let bytes = cid.to_bytes();
	assert!(bytes.len() <= CID_MAX_SIZE, "CID exceeds maximum size of {CID_MAX_SIZE} bytes");
	let mut raw = [0u8; CID_MAX_SIZE];
	raw[..bytes.len()].copy_from_slice(&bytes);
	raw
}

pub fn raw_to_cid(raw: &RawCid) -> Option<Cid> {
	Cid::read_bytes(Cursor::new(raw)).ok()
}
