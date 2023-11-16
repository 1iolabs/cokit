use crate::{
	crypto::{
		block::{Algorithm, EncryptedBlock, BLOCK_MULTICODEC},
		secret::Secret,
	},
	library::node_builder::{Node, NodeBuilder},
	AlgorithmError, Storage, StorageError,
};
use libipld::{cbor::DagCborCodec, Block, Cid, DefaultParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct EncryptedStorage<S> {
	key: Secret,
	algorithm: Algorithm,
	next: S,
	mapping: BlockMapping,
}
impl<S> EncryptedStorage<S>
where
	S: Storage,
{
	/// Create storage encryption layer.
	pub fn new(next: S, key: Secret, algorithm: Algorithm, map: Option<Cid>) -> Result<Self, StorageError> {
		// load mapping
		let mut mapping = BlockMapping::new();
		if let Some(mapping_root) = map {
			mapping.read(&next, &mapping_root)?;
		}

		// result
		Ok(Self { algorithm, key, mapping, next })
	}

	/// Get next storage layer.
	pub fn storage_mut(&mut self) -> &mut S {
		&mut self.next
	}

	/// Get next storage layer.
	pub fn storage(&self) -> &S {
		&self.next
	}

	/// Flush mapping to (parent) storage.
	///
	/// Note: This will not be encrypted and is intendet for local use only.
	pub fn flush_mapping(&mut self) -> Result<Cid, StorageError> {
		self.mapping.write(&mut self.next, Default::default())
	}

	/// This will regenerate and flush the encryption block mapping using supplied CIDs.
	pub fn regenerate_mapping(&mut self, cids: impl Iterator<Item = Cid>) -> Result<Cid, StorageError> {
		self.mapping = BlockMapping::from_cids(self, cids).map_err(|e| e.into())?;
		self.flush_mapping()
	}
}
impl<S> Storage for EncryptedStorage<S>
where
	S: Storage,
{
	/// Returns a block from storage.
	///
	/// Note: This expects the unencrypted CID.
	fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		match if cid.codec() == BLOCK_MULTICODEC { Some(cid) } else { self.mapping.map.get(cid) } {
			Some(encrypted_cid) => EncryptedBlock::try_from(self.next.get(encrypted_cid)?)
				.map_err(|_| StorageError::Internal)?
				.block(&self.key)
				.map_err(|_| StorageError::Internal),
			None => self.next.get(cid),
		}
	}

	/// Inserts a block into storage.
	///
	/// Note: This exepects the unencrypted Block.
	fn set(&mut self, block: Block<DefaultParams>) -> Result<(), StorageError> {
		let cid = block.cid().clone();

		// encrypt
		let encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|_| StorageError::Internal)?;
		let encrypted_block: Block<DefaultParams> = encrypted.try_into().map_err(|_| StorageError::Internal)?;
		let encrypted_cid = encrypted_block.cid().clone();

		// store
		self.next.set(encrypted_block)?;

		// map
		self.mapping.map.insert(cid, encrypted_cid);

		// result
		Ok(())
	}
}

#[derive(Debug, thiserror::Error)]
enum BlockMappingError {
	#[error("Storage Error")]
	Storage(#[from] StorageError),

	#[error("Algorithm Error")]
	Algorithm(#[from] AlgorithmError),
}
impl Into<StorageError> for BlockMappingError {
	fn into(self) -> StorageError {
		match self {
			BlockMappingError::Storage(e) => e,
			BlockMappingError::Algorithm(e) => match e {
				AlgorithmError::Cipher => StorageError::InvalidArgument, // likely wrong key supplied for given CID.
				AlgorithmError::InvalidArguments => StorageError::InvalidArgument,
				AlgorithmError::Decoding => StorageError::Internal,
				AlgorithmError::Encoding => StorageError::Internal,
				AlgorithmError::Size => StorageError::Internal,
			},
		}
	}
}

/// Serializeable block mapping.
/// This is used to store the mapping itself as an block.
#[derive(Debug, Serialize, Deserialize)]
struct BlockMapping {
	pub map: BTreeMap<Cid, Cid>,
}
impl BlockMapping {
	pub fn new() -> Self {
		Self { map: BTreeMap::new() }
	}

	/// Create new mapping by inspecting supplied CIDs.
	pub fn from_cids<S>(
		storage: &EncryptedStorage<S>,
		cids: impl Iterator<Item = Cid>,
	) -> Result<Self, BlockMappingError>
	where
		S: Storage,
	{
		let mut mapping = BlockMapping::new();
		for cid in cids {
			if cid.codec() == BLOCK_MULTICODEC {
				let encrypted_block: EncryptedBlock<DefaultParams> = storage.storage().get(&cid)?.try_into()?;
				mapping.map.insert(encrypted_block.cid(&storage.key)?, cid);
			}
		}
		Ok(mapping)
	}

	/// Read block mappings from `cid` via an block storage.
	pub fn read(&mut self, storage: &dyn Storage, cid: &Cid) -> Result<usize, StorageError> {
		let mut count = 0;

		// get block
		let block = storage.get(cid)?;
		if cid.codec() != Into::<u64>::into(DagCborCodec) {
			return Err(StorageError::InvalidArgument)
		}

		// get node
		let node: Node<(Cid, Cid)> =
			serde_ipld_dagcbor::from_slice(block.data()).map_err(|_| StorageError::InvalidArgument)?;

		// read
		match node {
			Node::Node(links) =>
				for link in links {
					let link_cid: Cid = link.into();
					count += self.read(storage, &link_cid)?;
				},
			Node::Leaf(entries) =>
				for (key, value) in entries.into_iter() {
					self.map.insert(key, value);
					count += 1;
				},
		}

		// result
		Ok(count)
	}

	/// Encode mapping into blocks.
	///
	/// Returns the root cid and all blocks.
	pub fn to_blocks(&self, options: WriteOptions) -> Result<(Cid, Vec<Block<DefaultParams>>), StorageError> {
		// validate
		if self.map.is_empty() {
			return Err(StorageError::InvalidArgument)
		}

		// blocks
		let mut builder = NodeBuilder::<(Cid, Cid)>::new(options.max_children);
		for (key, value) in self.map.iter() {
			builder.push((key.clone(), value.clone())).map_err(|e| e.into())?;
		}
		let blocks = builder.into_blocks().map_err(|e| e.into())?;
		let root_cid = blocks.get(0).expect("at least one block when have items").cid().clone();

		// result
		Ok((root_cid, blocks))
	}

	/// Write block mappings to an storage.
	///
	/// Returns `StorageError::InvalidArgument` when the map is empty.
	pub fn write(&self, storage: &mut dyn Storage, options: WriteOptions) -> Result<Cid, StorageError> {
		let (root_cid, blocks) = self.to_blocks(options)?;

		// store
		for block in blocks {
			storage.set(block)?
		}

		// result
		Ok(root_cid)
	}
}

struct WriteOptions {
	/// Max byte size for each block.
	// max_size: usize,

	/// Max children for each block.
	max_children: usize,
}
impl Default for WriteOptions {
	fn default() -> Self {
		Self {
			// max_size: 2 ^ 18,
			max_children: 174,
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		crypto::{block::Algorithm, secret::Secret},
		library::to_serialized_block::to_serialized_block,
		storage::{encrypted::EncryptedStorage, memory::MemoryStorage},
		types::storage::{Storage, StorageError},
	};
	use serde::{Deserialize, Serialize};
	use std::iter::repeat;

	#[derive(Debug, Serialize, Deserialize)]
	struct Test {
		hello: String,
	}

	#[test]
	fn roundtrip() {
		// storage
		let memory = MemoryStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let mut encryption = EncryptedStorage::new(memory, key, algorithm, None).unwrap();

		// block
		let data = Test { hello: "world".to_owned() };
		let block = to_serialized_block(&data, Default::default()).unwrap();

		// set
		encryption.set(block.clone()).unwrap();

		// get
		assert_eq!(block, encryption.get(block.cid()).unwrap());

		// validate that the CID dosn't exist in parent storage layer
		assert!(matches!(encryption.storage().get(block.cid()), Err(StorageError::NotFound)));
	}
}
