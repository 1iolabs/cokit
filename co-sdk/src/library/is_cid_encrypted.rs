use cid::Cid;
use co_primitives::{KnownMultiCodec, MultiCodec};

/// Return `true` if and of `cids` is encrypted.
pub fn is_cid_encrypted<'a>(cids: impl IntoIterator<Item = &'a Cid>) -> bool {
	for cid in cids {
		if MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			return true;
		}
	}
	return false;
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
		let storage = EncryptedBlockStorage::new(MemoryBlockStorage::new(), key, algorithm, Default::default());

		// set
		let cid = storage.set_serialized(&42).await.unwrap();
		let encrypted_cid = storage.content_mapping().to_plain(&cid).await.unwrap();
		assert_ne!(cid, encrypted_cid);
		assert_eq!(is_cid_encrypted([&cid]), false);
		assert_eq!(is_cid_encrypted([&encrypted_cid]), true);
	}
}
