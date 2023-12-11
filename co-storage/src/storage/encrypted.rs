use crate::{
	crypto::{
		block::{Algorithm, EncryptedBlock, BLOCK_MULTICODEC},
		secret::Secret,
	},
	library::node_builder::{DefaultNodeSerializer, Node, NodeBuilder, NodeBuilderError, NodeSerializer},
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
		let mut instance = Self { algorithm, key, mapping: BlockMapping::new(), next };

		// load mapping
		if let Some(mapping_root) = map {
			let mut mapping = BlockMapping::new();
			mapping.read(&mut instance, &mapping_root)?;
			instance.mapping = mapping;
		}

		// result
		Ok(instance)
	}

	/// Get next storage layer.
	pub fn storage_mut(&mut self) -> &mut S {
		&mut self.next
	}

	/// Get next storage layer.
	pub fn storage(&self) -> &S {
		&self.next
	}

	/// Consume storage and return next layer.
	pub fn into_storage(self) -> S {
		self.next
	}

	/// Flush mapping to (parent) storage.
	/// Returns the encrypted mapping CID.
	/// The mapping tree will also only link to encrypted CIDs.
	pub fn flush_mapping(&mut self) -> Result<Cid, StorageError> {
		let (root_cid, blocks) = self
			.mapping
			.to_blocks(EncryptedNodeSerializer::from_storage(self), Default::default())?;

		// store
		for block in blocks {
			self.storage_mut().set(block)?
		}

		// result
		Ok(root_cid)
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
	/// Note: This expects the unencrypted Block.
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
		if block.cid().codec() != Into::<u64>::into(DagCborCodec) {
			return Err(StorageError::InvalidArgument)
		}

		// get node
		let node: Node<(Cid, Cid)> =
			serde_ipld_dagcbor::from_slice(block.data()).map_err(|_| StorageError::InvalidArgument)?;

		// read
		match node {
			Node::Node(links) =>
				for link in links {
					count += self.read(storage, link.as_ref())?;
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
	pub fn to_blocks<S>(
		&self,
		serializer: S,
		options: WriteOptions,
	) -> Result<(Cid, Vec<Block<DefaultParams>>), StorageError>
	where
		S: NodeSerializer<(Cid, Cid)>,
	{
		// validate
		if self.map.is_empty() {
			return Err(StorageError::InvalidArgument)
		}

		// blocks
		let mut builder = NodeBuilder::<(Cid, Cid), S>::new(options.max_children, serializer);
		for (key, value) in self.map.iter() {
			builder.push((key.clone(), value.clone())).map_err(|e| e.into())?;
		}
		let blocks = builder.into_blocks().map_err(|e| e.into())?;
		let root_cid = blocks.get(0).expect("at least one block when have items").cid().clone();

		// result
		Ok((root_cid, blocks))
	}
}

/// Create encrypted block.
struct EncryptedNodeSerializer {
	key: Secret,
	algorithm: Algorithm,
}
impl EncryptedNodeSerializer {
	pub fn from_storage<S>(storage: &EncryptedStorage<S>) -> Self {
		Self { key: storage.key.clone(), algorithm: storage.algorithm }
	}
}
impl<T> NodeSerializer<T> for EncryptedNodeSerializer
where
	T: Clone + Serialize,
{
	fn serialize(&self, node: &Node<T>) -> Result<Block<DefaultParams>, NodeBuilderError> {
		let block = DefaultNodeSerializer::new().serialize(node)?;
		let encrypted = EncryptedBlock::encrypt(self.algorithm, &self.key, block)?;
		let encrypted_block: Block<DefaultParams> = encrypted.try_into()?;
		Ok(encrypted_block)
	}
}
impl From<AlgorithmError> for NodeBuilderError {
	fn from(_: AlgorithmError) -> Self {
		NodeBuilderError::Encoding
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
		crypto::{
			block::{Algorithm, BLOCK_MULTICODEC},
			secret::Secret,
		},
		storage::{encrypted::EncryptedStorage, memory::MemoryStorage},
		types::storage::{Storage, StorageError},
		BlockSerializer,
	};
	use libipld::{store::StoreParams, Cid, DefaultParams};
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
		let block = BlockSerializer::default().serialize(&data).unwrap();

		// set
		encryption.set(block.clone()).unwrap();

		// get
		assert_eq!(block, encryption.get(block.cid()).unwrap());

		// validate that the CID dosn't exist in parent storage layer
		assert!(matches!(encryption.storage().get(block.cid()), Err(StorageError::NotFound)));
	}

	#[test]
	fn store_mapping() {
		// storage
		let memory = MemoryStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let mut encryption = EncryptedStorage::new(memory, key.clone(), algorithm.clone(), None).unwrap();

		// blocks
		let mut cids: Vec<Cid> = Default::default();
		for i in 0..1024 {
			let data = Test { hello: format!("Hi {}!", i).to_owned() };
			let block = BlockSerializer::default().serialize(&data).unwrap();
			cids.push(block.cid().clone());
			encryption.set(block.clone()).unwrap();
		}

		// validate mapping
		let mapping_cid = encryption.flush_mapping().unwrap();
		assert_eq!(BLOCK_MULTICODEC, mapping_cid.codec()); // encrypted?

		// validate cids
		let memory: MemoryStorage = encryption.into_storage();
		let memory_cids: Vec<Cid> = memory.iter().cloned().collect();
		assert_eq!(7 + 1024, memory_cids.len()); // 7 (merkle) mapping blocks and 1024 data blocks
		for memory_cid in memory_cids.iter() {
			let memory_block = memory.get(memory_cid).unwrap();
			assert_eq!(BLOCK_MULTICODEC, memory_cid.codec()); // all blocks are encrypted
			assert!(DefaultParams::MAX_BLOCK_SIZE > memory_block.data().len()); // all blocks fit in max block size
		}

		// validate load blocks again
		let encryption = EncryptedStorage::new(memory, key, algorithm, Some(mapping_cid)).unwrap();
		for cid in cids {
			encryption.get(&cid).unwrap();
		}
	}
}
