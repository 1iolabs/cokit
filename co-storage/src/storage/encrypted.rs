use crate::{
	crypto::{
		block::{Algorithm, EncryptedBlock, BLOCK_MULTICODEC},
		secret::Secret,
	},
	AlgorithmError, BlockStat, BlockStorage, BlockStorageContentMapping, Storage, StorageContentMapping, StorageError,
};
use async_trait::async_trait;
use co_primitives::{
	from_cbor, DefaultNodeSerializer, MultiCodec, Node, NodeBuilder, NodeBuilderError, NodeSerializer,
};
use futures::{stream::FuturesOrdered, StreamExt};
use libipld::{store::StoreParams, Block, Cid};
use serde::{Deserialize, Serialize};
use std::{
	borrow::{Borrow, Cow},
	collections::BTreeMap,
	sync::Arc,
};
use tokio::sync::RwLock;

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
impl<S> StorageContentMapping for EncryptedStorage<S> {
	fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		if mapped.codec() == BLOCK_MULTICODEC {
			self.mapping.get(mapped)
		} else {
			None
		}
	}

	fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.mapping.get_first_by_value(plain)
	}
}
impl<S> EncryptedStorage<S>
where
	S: Storage,
{
	/// Load mapping from CID.
	pub fn load_mapping(&mut self, map: &Cid) -> Result<(), StorageError> {
		let mut mapping = BlockMapping::new();
		mapping.read_mappings_storage(self, map)?;
		self.mapping = mapping;
		Ok(())
	}

	/// Flush mapping to (parent) storage.
	/// Returns the encrypted mapping CID.
	/// The mapping tree will also only link to encrypted CIDs.
	pub fn flush_mapping(&mut self) -> Result<Option<Cid>, StorageError> {
		// serializer
		let node_serializer = EncryptedNodeSerializer { algorithm: self.algorithm, key: self.key.clone() };

		// blocks
		let blocks = self.mapping.to_blocks(node_serializer, Default::default())?;

		// store
		let mut root = None;
		for block in blocks {
			let result = self.storage_mut().set(block)?;
			if root.is_none() {
				root = Some(result);
			}
		}

		// result
		Ok(root)
	}

	/// This will regenerate and flush the encryption block mapping using supplied CIDs.
	pub fn regenerate_mapping(&mut self, cids: impl Iterator<Item = Cid>) -> Result<Option<Cid>, StorageError> {
		self.mapping = BlockMapping::from_cids_storage(self, cids).map_err(|e| StorageError::Internal(e.into()))?;
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
	/// Note: As the API is transparent this expects the unencrypted Block and returns the unencrypted CID.
	fn set(&mut self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let cid = *block.cid();

		// encrypt
		let encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|e| StorageError::Internal(e.into()))?;
		let encrypted_block: Block<Self::StoreParams> = encrypted
			.try_into()
			.map_err(|e: AlgorithmError| StorageError::Internal(e.into()))?;

		// store
		let encrypted_cid = self.next.set(encrypted_block)?;

		// map
		self.mapping.insert(cid, encrypted_cid);

		// result
		Ok(cid)
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
impl<S> EncryptedBlockStorage<S>
where
	S: BlockStorage + Send + Sync,
{
	pub fn new(next: S, key: Secret, algorithm: Algorithm) -> Self {
		Self { algorithm, key, mapping: Default::default(), next }
	}

	/// Load mapping from CID.
	/// This will add the mappings to the existing.
	pub async fn load_mapping(&mut self, map: &Cid) -> Result<(), StorageError> {
		self.mapping.write().await.read_mappings(self, map).await?;
		Ok(())
	}

	/// Flush mapping to (parent) storage.
	/// Returns the encrypted mapping CID.
	/// The mapping tree will also only link to encrypted CIDs.
	pub async fn flush_mapping(&self) -> Result<Option<Cid>, StorageError> {
		// serializer
		let node_serializer = EncryptedNodeSerializer { algorithm: self.algorithm, key: self.key.clone() };

		// blocks
		let blocks = self.mapping.read().await.to_blocks(node_serializer, Default::default())?;

		// store
		let mut root = None;
		for block in blocks {
			let result = self.next.set(block).await?;
			if root.is_none() {
				root = Some(result)
			}
			// TODO: PIN/UNPIN
		}

		// log
		#[cfg(debug_assertions)]
		tracing::trace!(?root, "storage-flush-mapping");

		// result
		Ok(root)
	}

	/// This will regenerate and flush the encryption block mapping using supplied CIDs.
	pub async fn regenerate_mapping(&mut self, cids: impl Iterator<Item = Cid>) -> Result<Option<Cid>, StorageError> {
		self.mapping = Arc::new(RwLock::new(
			BlockMapping::from_cids(self, cids)
				.await
				.map_err(|e| StorageError::Internal(e.into()))?,
		));
		self.flush_mapping().await
	}

	// Create BlockStorageContentMapping instance.
	pub fn content_mapping(&self) -> EncryptedBlockStorageMapping {
		EncryptedBlockStorageMapping { mapping: self.mapping.clone() }
	}
}
#[async_trait]
impl<S> BlockStorage for EncryptedBlockStorage<S>
where
	S: BlockStorage + Send + Sync,
{
	type StoreParams = S::StoreParams;

	/// Get block.
	///
	/// This decrypts transparently. If an encrypted CID is specified the unencrypted block will be returned.
	/// If an unencrypted CID is specified that could not be mapped it will be forwarded to next layer.
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

	#[tracing::instrument(err, skip(self, block), fields(cid = ?block.cid()))]
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let cid = *block.cid();

		// encrypt
		let encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|e| StorageError::Internal(e.into()))?;
		let encrypted_block: Block<Self::StoreParams> = encrypted
			.try_into()
			.map_err(|e: AlgorithmError| StorageError::Internal(e.into()))?;

		// store
		let encrypted_cid = self.next.set(encrypted_block).await?;

		// map
		self.mapping.write().await.insert(cid, encrypted_cid);

		// trace (only in debug because this has security implications)
		#[cfg(debug_assertions)]
		tracing::trace!(?cid, ?encrypted_cid, "storage-set");

		// result
		Ok(cid)
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

#[derive(Debug, Clone)]
pub struct EncryptedBlockStorageMapping {
	mapping: Arc<RwLock<BlockMapping>>,
}
impl EncryptedBlockStorageMapping {
	/// Load mapping from CID.
	/// This will add the mappings to the existing.
	pub async fn load_mapping<S: BlockStorage>(&self, storage: &S, map: &Cid) -> Result<(), StorageError> {
		self.mapping.write().await.read_mappings(storage, map).await?;
		Ok(())
	}
}
#[async_trait]
impl BlockStorageContentMapping for EncryptedBlockStorageMapping {
	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.mapping.read().await.get(mapped)
	}

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.mapping.read().await.get_first_by_value(plain)
	}
}

#[derive(Debug, thiserror::Error)]
enum BlockMappingError {
	#[error("Storage Error")]
	Storage(#[from] StorageError),

	#[error("Algorithm Error")]
	Algorithm(#[from] AlgorithmError),
}
impl From<BlockMappingError> for StorageError {
	fn from(val: BlockMappingError) -> Self {
		match val {
			BlockMappingError::Storage(e) => e,
			BlockMappingError::Algorithm(e) => match e {
				AlgorithmError::Cipher => StorageError::InvalidArgument(e.into()), /* likely wrong key supplied for */
				// given CID.
				AlgorithmError::InvalidArguments(_) => StorageError::InvalidArgument(e.into()),
				AlgorithmError::Decoding => StorageError::Internal(e.into()),
				AlgorithmError::Encoding => StorageError::Internal(e.into()),
				AlgorithmError::Size => StorageError::Internal(e.into()),
			},
		}
	}
}

/// Serializeable block mapping.
/// This is used to store the mapping itself as an block.
#[derive(Debug, Serialize, Deserialize, Default)]
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

	pub fn get_first_by_value(&self, value: &Cid) -> Option<Cid> {
		self.map.iter().find_map(|(k, v)| {
			if v == value {
				return Some(*k);
			}
			None
		})
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
		MultiCodec::with_dag_cbor(block.cid())?;

		// get node
		let node: Node<(Cid, Cid)> = from_cbor(block.data()).map_err(|e| StorageError::InvalidArgument(e.into()))?;

		// read
		match node {
			Node::Node(links) => {
				for link in links {
					count += self.read_mappings_storage(storage, link.as_ref())?;
				}
			},
			Node::Leaf(entries) => {
				for (key, value) in entries.into_iter() {
					self.insert(key, value);
					count += 1;
				}
			},
		}

		// result
		Ok(count)
	}

	/// Read block mappings from `cid` via an block storage.
	/// Idempotency: Yes
	pub async fn read_mappings<S: BlockStorage>(&mut self, storage: &S, cid: &Cid) -> Result<usize, StorageError> {
		let mut count = 0;
		let mut tasks = FuturesOrdered::new();

		// first
		let read = |cid: Cid| async move { storage.get(&cid).await };
		tasks.push_back(read(*cid));

		// work
		while let Some(block) = tasks.next().await {
			let block = block?;

			// validate
			MultiCodec::with_dag_cbor(block.cid())?;

			// get node
			let node: Node<(Cid, Cid)> =
				from_cbor(block.data()).map_err(|e| StorageError::InvalidArgument(e.into()))?;

			// read
			match node {
				Node::Node(links) => {
					for link in links {
						tasks.push_back(read(link.into()));
					}
				},
				Node::Leaf(entries) => {
					for (key, value) in entries.into_iter() {
						self.insert(key, value);
						count += 1;
					}
				},
			}
		}

		// result
		Ok(count)
	}

	/// Encode mapping into blocks.
	///
	/// Returns the root cid and all blocks.
	/// The first block retuned is the root.
	pub fn to_blocks<S, P: StoreParams>(
		&self,
		serializer: S,
		options: WriteOptions,
	) -> Result<Vec<Block<P>>, StorageError>
	where
		S: NodeSerializer<(Cid, Cid), P>,
	{
		// blocks
		let mut builder = NodeBuilder::<(Cid, Cid), S, P>::new(options.max_children, serializer);
		for (key, value) in self.map.iter() {
			builder.push((*key, *value)).map_err(|e| StorageError::Internal(e.into()))?;
		}
		let blocks = builder.into_blocks().map_err(|e| StorageError::Internal(e.into()))?;

		// result
		Ok(blocks)
	}
}

/// Create encrypted block.
struct EncryptedNodeSerializer {
	key: Secret,
	algorithm: Algorithm,
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
		BlockStorage, EncryptedBlockStorage, MemoryBlockStorage,
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
	fn roundtrip_storage() {
		// storage
		let memory = MemoryStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let mut encryption = EncryptedStorage::new(memory, key, algorithm);

		// block
		let data = Test { hello: "world".to_owned() };
		let block = BlockSerializer::default().serialize(&data).unwrap();

		// set
		let result = encryption.set(block.clone()).unwrap();
		assert_eq!(&result, block.cid());

		// get
		assert_eq!(encryption.get(block.cid()).unwrap(), block);

		// validate that the CID dosn't exist in parent storage layer
		assert!(matches!(encryption.storage().get(block.cid()), Err(StorageError::NotFound(_, _))));
	}

	#[tokio::test]
	async fn roundtrip() {
		// storage
		let memory = MemoryBlockStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let encryption = EncryptedBlockStorage::new(memory.clone(), key, algorithm);

		// block
		let data = Test { hello: "world".to_owned() };
		let block = BlockSerializer::default().serialize(&data).unwrap();

		// set
		let result = encryption.set(block.clone()).await.unwrap();
		assert_eq!(&result, block.cid());

		// get
		assert_eq!(encryption.get(block.cid()).await.unwrap(), block);

		// validate that the CID dosn't exist in parent storage layer
		assert!(matches!(memory.get(block.cid()).await, Err(StorageError::NotFound(_, _))));
	}

	#[test]
	fn store_mapping() {
		// storage
		let memory = MemoryStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let mut encryption = EncryptedStorage::new(memory, key.clone(), algorithm);

		// blocks
		let mut cids: Vec<Cid> = Default::default();
		for i in 0..1024 {
			let data = Test { hello: format!("Hi {}!", i).to_owned() };
			let block = BlockSerializer::default().serialize(&data).unwrap();
			cids.push(*block.cid());
			encryption.set(block.clone()).unwrap();
		}

		// validate mapping
		let mapping_cid = encryption.flush_mapping().unwrap().expect("Mappings if we have items");
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
