// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_primitives::Block;

/// TODO remove
pub fn to_libipld_cid(cid: cid::Cid) -> libipld::Cid {
	// let version = match cid.version() {
	// 	cid::Version::V0 => libipld::cid::Version::V0,
	// 	cid::Version::V1 => libipld::cid::Version::V1,
	// };
	// libipld::Cid::new(version, cid.codec(), cid.hash().clone()).unwrap()
	libipld::Cid::try_from(cid.to_bytes()).unwrap()
}

/// TODO remove
pub fn from_libipld_cid(cid: libipld::Cid) -> cid::Cid {
	// let version = match cid.version() {
	// 	cid::Version::V0 => libipld::cid::Version::V0,
	// 	cid::Version::V1 => libipld::cid::Version::V1,
	// };
	// libipld::Cid::new(version, cid.codec(), cid.hash().clone()).unwrap()
	cid::Cid::try_from(cid.to_bytes()).unwrap()
}

/// TODO remove
#[allow(dead_code)]
pub fn to_libipld_block<S: libipld::store::StoreParams>(block: Block) -> libipld::Block<S> {
	let (cid, data) = block.into_inner();
	libipld::Block::new_unchecked(to_libipld_cid(cid), data)
}

/// TODO remove
pub fn from_libipld_block<S: libipld::store::StoreParams>(block: libipld::Block<S>) -> Block {
	let (cid, data) = block.into_inner();
	Block::new_unchecked(from_libipld_cid(cid), data)
}
