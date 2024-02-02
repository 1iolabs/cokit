use crate::{
	crypto::{
		block::{Algorithm, EncryptedBlock, BLOCK_MULTICODEC},
		secret::Secret,
	},
	AlgorithmError, BlockStat, BlockStorage, Storage, StorageError,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::{DefaultNodeSerializer, Node, NodeBuilder, NodeBuilderError, NodeSerializer};
use futures::{stream::FuturesOrdered, StreamExt};
use libipld::{cbor::DagCborCodec, store::StoreParams, Block, Cid};
use serde::{Deserialize, Serialize};
use std::{
	borrow::{Borrow, Cow},
	collections::BTreeMap,
	sync::Arc,
};
use tokio::sync::RwLock;

pub trait Encryption {
	fn key(&self) -> &Secret;
	fn algorithm(&self) -> Algorithm;
}

#[derive(Debug)]
pub struct EncryptedStorage<S> {
	key: Secret,
	algorithm: Algorithm,
	next: S,
	mapping: BlockMapping,
}
impl<S> EncryptedStorage<S> {
	pub fn new(next: S, key: Secret, algorithm: Algorithm) -> Self {
		Self { algorithm, key, mapping: BlockMapping::new(), next }
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
}
impl<S> Encryption for EncryptedStorage<S> {
	fn key(&self) -> &Secret {
		&self.key
	}

	fn algorithm(&self) -> Algorithm {
		self.algorithm
	}
}
impl<S> EncryptedStorage<S>
where
	S: Storage,
{
	/// Load mapping from CID.
	pub fn load_mapping(&mut self, map: &Cid) -> Result<(), StorageError> {
		let mut mapping = BlockMapping::new();
		mapping.read_mappings_storage(self, &map)?;
		self.mapping = mapping;
		Ok(())
	}

	/// Flush mapping to (parent) storage.
	/// Returns the encrypted mapping CID.
	/// The mapping tree will also only link to encrypted CIDs.
	pub fn flush_mapping(&mut self) -> Result<Cid, StorageError> {
		let encryption: &dyn Encryption = self;
		let (root_cid, blocks) = self
			.mapping
			.to_blocks(EncryptedNodeSerializer::from(encryption), Default::default())?;

		// store
		for block in blocks {
			self.storage_mut().set(block)?;
		}

		// result
		Ok(root_cid)
	}

	/// This will regenerate and flush the encryption block mapping using supplied CIDs.
	pub fn regenerate_mapping(&mut self, cids: impl Iterator<Item = Cid>) -> Result<Cid, StorageError> {
		self.mapping = BlockMapping::from_cids_storage(self, cids).map_err(|e| e.into())?;
		self.flush_mapping()
	}
}
impl<S> Storage for EncryptedStorage<S>
where
	S: Storage,
{
	type StoreParams = S::StoreParams;

	/// Returns a block from storage.
	///
	/// Note: This expects the unencrypted CID.
	fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		match if cid.codec() == BLOCK_MULTICODEC { Some(cid) } else { self.mapping.map.get(cid) } {
			Some(encrypted_cid) => EncryptedBlock::try_from(self.next.get(encrypted_cid)?)
				.map_err(|e| StorageError::Internal(e.into()))?
				.block(&self.key)
				.map_err(|e| StorageError::Internal(e.into())),
			None => self.next.get(cid),
		}
	}

	/// Inserts a block into storage.
	///
	/// Note: This expects the unencrypted Block.
	fn set(&mut self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let cid = block.cid().clone();

		// encrypt
		let encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|e| StorageError::Internal(e.into()))?;
		let encrypted_block: Block<Self::StoreParams> = encrypted
			.try_into()
			.map_err(|e: AlgorithmError| StorageError::Internal(e.into()))?;
		let encrypted_cid = encrypted_block.cid().clone();

		// store
		let result = self.next.set(encrypted_block)?;

		// map
		self.mapping.insert(cid, encrypted_cid);

		// result
		Ok(result)
	}

	/// Remove a block from storage.
	///
	/// Note: This expects the unencrypted CID.
	fn remove(&mut self, cid: &Cid) -> Result<(), StorageError> {
		match if cid.codec() == BLOCK_MULTICODEC { Some(cid) } else { self.mapping.map.get(cid) } {
			Some(encrypted_cid) => self.next.remove(encrypted_cid),
			None => self.next.remove(cid),
		}
	}
}

#[derive(Debug, Clone)]
pub struct EncryptedBlockStorage<S> {
	key: Secret,
	algorithm: Algorithm,
	next: S,
	mapping: Arc<RwLock<BlockMapping>>,
}
impl<S> Encryption for EncryptedBlockStorage<S> {
	fn key(&self) -> &Secret {
		&self.key
	}

	fn algorithm(&self) -> Algorithm {
		self.algorithm
	}
}
impl<S> EncryptedBlockStorage<S>
where
	S: BlockStorage + Send + Sync,
{
	pub fn new(next: S, key: Secret) -> Self {
		Self { algorithm: Default::default(), key, mapping: Default::default(), next }
	}

	/// Load mapping from CID.
	pub async fn load_mapping(&mut self, map: &Cid) -> Result<(), StorageError> {
		let mut mapping = BlockMapping::new();
		mapping.read_mappings(self, &map).await?;
		self.mapping = Arc::new(RwLock::new(mapping));
		Ok(())
	}

	/// Flush mapping to (parent) storage.
	/// Returns the encrypted mapping CID.
	/// The mapping tree will also only link to encrypted CIDs.
	pub async fn flush_mapping(&mut self) -> Result<Cid, StorageError> {
		let encryption: &dyn Encryption = self;
		let (root_cid, blocks) = self
			.mapping
			.read()
			.await
			.to_blocks(EncryptedNodeSerializer::from(encryption), Default::default())?;

		// store
		for block in blocks {
			self.next.set(block).await?;
			// TODO: PIN/UNPIN
		}

		// result
		Ok(root_cid)
	}

	/// This will regenerate and flush the encryption block mapping using supplied CIDs.
	pub async fn regenerate_mapping(&mut self, cids: impl Iterator<Item = Cid>) -> Result<Cid, StorageError> {
		self.mapping = Arc::new(RwLock::new(BlockMapping::from_cids(self, cids).await.map_err(|e| e.into())?));
		self.flush_mapping().await
	}
}
#[async_trait]
impl<S> BlockStorage for EncryptedBlockStorage<S>
where
	S: BlockStorage + Send + Sync,
{
	type StoreParams = S::StoreParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let mapped_cid: Option<Cow<'_, Cid>> = if cid.codec() == BLOCK_MULTICODEC {
			Some(cid.into())
		} else {
			self.mapping.read().await.get(cid).map(|e| e.into())
		};
		match mapped_cid.borrow() {
			Some(encrypted_cid) => EncryptedBlock::try_from(self.next.get(encrypted_cid).await?)
				.map_err(|e| StorageError::Internal(e.into()))?
				.block(&self.key)
				.map_err(|e| StorageError::Internal(e.into())),
			None => self.next.get(cid).await,
		}
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let cid = block.cid().clone();

		// encrypt
		let encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|e| StorageError::Internal(e.into()))?;
		let encrypted_block: Block<Self::StoreParams> = encrypted
			.try_into()
			.map_err(|e: AlgorithmError| StorageError::Internal(e.into()))?;
		let encrypted_cid = encrypted_block.cid().clone();

		// store
		let result = self.next.set(encrypted_block).await?;

		// map
		// TODO: Make Sync?
		self.mapping.write().await.insert(cid, encrypted_cid);

		// result
		Ok(result)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		let mapped_cid: Option<Cow<'_, Cid>> = if cid.codec() == BLOCK_MULTICODEC {
			Some(cid.into())
		} else {
			self.mapping.read().await.get(cid).map(|e| e.into())
		};
		match mapped_cid.borrow() {
			Some(encrypted_cid) => self.next.remove(encrypted_cid).await,
			None => self.next.remove(cid).await,
		}
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.get(cid).await.map(|v| BlockStat { size: v.data().len() as u64 })
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
				AlgorithmError::Cipher => StorageError::InvalidArgument(e.into()), /* likely wrong key supplied for */
				// given CID.
				AlgorithmError::InvalidArguments => StorageError::InvalidArgument(e.into()),
				AlgorithmError::Decoding => StorageError::Internal(e.into()),
				AlgorithmError::Encoding => StorageError::Internal(e.into()),
				AlgorithmError::Size => StorageError::Internal(e.into()),
			},
		}
	}
}

/// Serializeable block mapping.
/// This is used to store the mapping itself as an block.
#[derive(Debug, Serialize, Deserialize)]
struct BlockMapping {
	map: BTreeMap<Cid, Cid>,
}
impl BlockMapping {
	pub fn new() -> Self {
		Self { map: BTreeMap::new() }
	}

	pub fn get(&self, key: &Cid) -> Option<Cid> {
		self.map.get(key).cloned()
	}

	pub fn insert(&mut self, key: Cid, value: Cid) -> Option<Cid> {
		self.map.insert(key, value)
	}

	/// Create new mapping by inspecting supplied CIDs.
	pub fn from_cids_storage<S>(
		storage: &EncryptedStorage<S>,
		cids: impl Iterator<Item = Cid>,
	) -> Result<Self, BlockMappingError>
	where
		S: Storage,
	{
		let mut mapping = BlockMapping::new();
		for cid in cids {
			if cid.codec() == BLOCK_MULTICODEC {
				let encrypted_block: EncryptedBlock<S::StoreParams> = storage.storage().get(&cid)?.try_into()?;
				mapping.insert(encrypted_block.cid(&storage.key)?, cid);
			}
		}
		Ok(mapping)
	}

	pub async fn from_cids<S>(
		storage: &EncryptedBlockStorage<S>,
		cids: impl Iterator<Item = Cid>,
	) -> Result<Self, BlockMappingError>
	where
		S: BlockStorage,
	{
		let mut mapping = BlockMapping::new();
		for cid in cids {
			if cid.codec() == BLOCK_MULTICODEC {
				let encrypted_block: EncryptedBlock<S::StoreParams> = storage.next.get(&cid).await?.try_into()?;
				mapping.insert(encrypted_block.cid(&storage.key)?, cid);
			}
		}
		Ok(mapping)
	}

	/// Read block mappings from `cid` via an block storage.
	pub fn read_mappings_storage<S: Storage>(&mut self, storage: &S, cid: &Cid) -> Result<usize, StorageError> {
		let mut count = 0;

		// get block
		let block = storage.get(cid)?;
		if block.cid().codec() != Into::<u64>::into(DagCborCodec) {
			return Err(StorageError::InvalidArgument(anyhow!("Invalid codec")))
		}

		// get node
		let node: Node<(Cid, Cid)> =
			serde_ipld_dagcbor::from_slice(block.data()).map_err(|e| StorageError::InvalidArgument(e.into()))?;

		// read
		match node {
			Node::Node(links) =>
				for link in links {
					count += self.read_mappings_storage(storage, link.as_ref())?;
				},
			Node::Leaf(entries) =>
				for (key, value) in entries.into_iter() {
					self.insert(key, value);
					count += 1;
				},
		}

		// result
		Ok(count)
	}

	/// Read block mappings from `cid` via an block storage.
	pub async fn read_mappings<S: BlockStorage>(&mut self, storage: &S, cid: &Cid) -> Result<usize, StorageError> {
		let mut count = 0;
		let mut tasks = FuturesOrdered::new();

		// first
		let read = |cid: Cid| async move { storage.get(&cid).await };
		tasks.push_back(read(cid.clone()));

		// work
		while let Some(block) = tasks.next().await {
			let block = block?;

			// validate
			if block.cid().codec() != Into::<u64>::into(DagCborCodec) {
				return Err(StorageError::InvalidArgument(anyhow!("Invalid codec")))
			}

			// get node
			let node: Node<(Cid, Cid)> =
				serde_ipld_dagcbor::from_slice(block.data()).map_err(|e| StorageError::InvalidArgument(e.into()))?;

			// read
			match node {
				Node::Node(links) =>
					for link in links {
						tasks.push_back(read(link.into()));
					},
				Node::Leaf(entries) =>
					for (key, value) in entries.into_iter() {
						self.insert(key, value);
						count += 1;
					},
			}
		}

		// result
		Ok(count)
	}

	/// Encode mapping into blocks.
	///
	/// Returns the root cid and all blocks.
	pub fn to_blocks<S, P: StoreParams>(
		&self,
		serializer: S,
		options: WriteOptions,
	) -> Result<(Cid, Vec<Block<P>>), StorageError>
	where
		S: NodeSerializer<(Cid, Cid), P>,
	{
		// validate
		if self.map.is_empty() {
			return Err(StorageError::InvalidArgument(anyhow!("Empty")))
		}

		// blocks
		let mut builder = NodeBuilder::<(Cid, Cid), S, P>::new(options.max_children, serializer);
		for (key, value) in self.map.iter() {
			builder
				.push((key.clone(), value.clone()))
				.map_err(|e| StorageError::Internal(e.into()))?;
		}
		let blocks = builder.into_blocks().map_err(|e| StorageError::Internal(e.into()))?;
		let root_cid = blocks.get(0).expect("at least one block when have items").cid().clone();

		// result
		Ok((root_cid, blocks))
	}
}
impl Default for BlockMapping {
	fn default() -> Self {
		Self { map: Default::default() }
	}
}

/// Create encrypted block.
struct EncryptedNodeSerializer {
	key: Secret,
	algorithm: Algorithm,
}
impl From<&dyn Encryption> for EncryptedNodeSerializer {
	fn from(value: &dyn Encryption) -> Self {
		Self { key: value.key().clone(), algorithm: value.algorithm() }
	}
}
impl<T, P> NodeSerializer<T, P> for EncryptedNodeSerializer
where
	T: Clone + Serialize,
	P: StoreParams,
{
	fn serialize(&self, node: &Node<T>) -> Result<Block<P>, NodeBuilderError> {
		let block = DefaultNodeSerializer::new().serialize(node)?;
		let encrypted = EncryptedBlock::encrypt(self.algorithm, &self.key, block)?;
		let encrypted_block: Block<P> = encrypted.try_into()?;
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
	};
	use co_primitives::BlockSerializer;
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
		let mut encryption = EncryptedStorage::new(memory, key, algorithm);

		// block
		let data = Test { hello: "world".to_owned() };
		let block = BlockSerializer::default().serialize(&data).unwrap();

		// set
		encryption.set(block.clone()).unwrap();

		// get
		assert_eq!(encryption.get(block.cid()).unwrap(), block);

		// validate that the CID dosn't exist in parent storage layer
		assert!(matches!(encryption.storage().get(block.cid()), Err(StorageError::NotFound(_))));
	}

	#[test]
	fn store_mapping() {
		// storage
		let memory = MemoryStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let mut encryption = EncryptedStorage::new(memory, key.clone(), algorithm.clone());

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
		assert_eq!(mapping_cid.codec(), BLOCK_MULTICODEC); // encrypted?

		// validate cids
		let memory: MemoryStorage = encryption.into_storage();
		let memory_cids: Vec<Cid> = memory.iter().cloned().collect();
		assert_eq!(memory_cids.len(), 7 + 1024); // 7 (merkle) mapping blocks and 1024 data blocks
		for memory_cid in memory_cids.iter() {
			let memory_block = memory.get(memory_cid).unwrap();
			assert_eq!(memory_cid.codec(), BLOCK_MULTICODEC); // all blocks are encrypted
			assert!(DefaultParams::MAX_BLOCK_SIZE > memory_block.data().len()); // all blocks fit in max block size
		}

		// validate load blocks again
		let mut encryption = EncryptedStorage::new(memory, key, algorithm);
		encryption.load_mapping(&mapping_cid).unwrap();
		for cid in cids {
			encryption.get(&cid).unwrap();
		}
	}
}
