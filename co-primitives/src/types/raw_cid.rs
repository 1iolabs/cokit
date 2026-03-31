// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
