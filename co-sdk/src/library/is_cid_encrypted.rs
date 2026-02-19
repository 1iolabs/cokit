// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_primitives::{KnownMultiCodec, MultiCodec};
use std::borrow::Borrow;

/// Return `true` if and of `cids` is encrypted.
pub fn is_cid_encrypted<C>(cids: impl IntoIterator<Item = C>) -> bool
where
	C: Borrow<Cid>,
{
	for cid in cids {
		if MultiCodec::is(cid.borrow(), KnownMultiCodec::CoEncryptedBlock) {
			return true;
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use crate::library::is_cid_encrypted::is_cid_encrypted;
	use co_primitives::BlockStorageExt;
	use co_storage::{Algorithm, BlockStorageContentMapping, EncryptedBlockStorage, MemoryBlockStorage};

	#[tokio::test]
	async fn test_is_cid_encrypted() {
		// storage
		let algorithm = Algorithm::default();
		let key = algorithm.generate_serect();
		let storage = EncryptedBlockStorage::new(MemoryBlockStorage::default(), key, algorithm, Default::default());

		// set
		let cid = storage.set_serialized(&42).await.unwrap();
		let encrypted_cid = storage.to_plain(&cid).await.unwrap();
		assert_ne!(cid, encrypted_cid);
		assert!(!is_cid_encrypted([&cid]));
		assert!(is_cid_encrypted([&encrypted_cid]));
	}
}
