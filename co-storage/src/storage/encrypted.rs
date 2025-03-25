use crate::{
	crypto::{
		block::{Algorithm, BlockPayload, EncryptedBlock, Header, BLOCK_MULTICODEC},
		secret::Secret,
	},
	AlgorithmError, BlockStorageContentMapping, StorageError,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	from_cbor, Block, BlockLinks, BlockStat, BlockStorage, BlockStorageSettings, CloneWithBlockStorageSettings,
	DefaultNodeSerializer, KnownMultiCodec, Link, MultiCodec, Node, NodeBuilder, NodeBuilderError, NodeSerializer,
	StoreParams,
};
use futures::{
	stream::{self, FuturesOrdered},
	StreamExt, TryStreamExt,
};
use serde::Serialize;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct EncryptedBlockStorage<S> {
	key: Secret,
	algorithm: Algorithm,
	next: S,
	mapping: EncryptedBlockStorageMapping,
	links: BlockLinks,
	reference_mode: EncryptionReferenceMode,
}
impl<S> EncryptedBlockStorage<S>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
{
	pub fn new(next: S, key: Secret, algorithm: Algorithm, mapping: EncryptedBlockStorageMapping) -> Self {
		Self { algorithm, key, mapping, next, links: Default::default(), reference_mode: Default::default() }
	}

	pub fn with_encryption_reference_mode(mut self, mode: EncryptionReferenceMode) -> Self {
		self.reference_mode = mode;
		self
	}

	/// Get next storage.
	pub fn storage(&self) -> &S {
		&self.next
	}

	/// Set next storage.
	pub fn set_storage(&mut self, next: S) {
		self.next = next;
	}

	/// Load mapping from CID.
	/// This will add the mappings to the existing.
	pub async fn load_mapping(&self, map: &Cid) -> Result<(), StorageError> {
		self.mapping.load_mapping(self, map).await?;
		Ok(())
	}

	/// Flush mapping to (parent) storage.
	/// Returns the encrypted mapping CID.
	/// The mapping tree will also only link to encrypted CIDs.
	pub async fn flush_mapping(&self) -> Result<Option<Cid>, StorageError> {
		// serializer
		let node_serializer = EncryptedNodeSerializer { algorithm: self.algorithm, key: self.key.clone() };

		// blocks
		let (root, blocks) = self.mapping.to_blocks(node_serializer, Default::default()).await?;

		// store
		for block in blocks {
			self.next.set(block).await?;
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
		self.mapping
			.set_mapping(
				BlockMapping::from_cids(self, cids)
					.await
					.map_err(|e| StorageError::Internal(e.into()))?,
			)
			.await;
		self.flush_mapping().await
	}

	// Create BlockStorageContentMapping instance.
	pub fn content_mapping(&self) -> EncryptedBlockStorageMapping {
		self.mapping.clone()
	}

	/// Insert mapping for encrypted block.
	/// Returns true if mapping has been changed.
	#[deprecated]
	pub async fn insert_mapping(&self, encrypted: &Cid, plain: Option<&Cid>) -> Result<bool, StorageError> {
		if MultiCodec::is(encrypted, KnownMultiCodec::CoEncryptedBlock) {
			let plain = match plain {
				Some(plain) => *plain,
				None => *self.get_unencrypted(encrypted).await?.cid(),
			};
			let old = self.mapping.insert(plain, *encrypted).await;
			Ok(old.is_none() || old.as_ref() != Some(encrypted))
		} else {
			Ok(false)
		}
	}

	/// Get encrypted cid as unencrypted block.
	pub async fn get_unencrypted(&self, cid: &Cid) -> Result<Block<S::StoreParams>, StorageError> {
		Ok(if MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			// get block
			let mut block =
				EncryptedBlock::try_from(self.next.get(&cid).await?).map_err(|e| StorageError::Internal(e.into()))?;

			// make inline
			if let Some(blocks) = block.payload.blocks() {
				let blocks = stream::iter(blocks.into_iter().cloned())
					.map(|cid| {
						let next = self.next.clone();
						async move { next.get(&cid).await }
					})
					.buffered(10)
					.try_collect::<Vec<_>>()
					.await?
					.into_iter()
					.map(|block| block.into_inner());
				block
					.payload
					.try_inline_blocks(blocks)
					.map_err(|_| StorageError::Internal(anyhow!("Inline blocks failed")))?;
			}

			// decrypt
			let plain = block.block(&self.key).map_err(|e| StorageError::Internal(e.into()))?;

			// apply mappings
			if !plain.references.is_empty() {
				self.mapping.extend(plain.references.iter().map(|(k, v)| (*k, *v))).await;
			}

			// result
			plain.into()
		} else {
			self.next.get(cid).await?
		})
	}

	/// Set encrypted block.
	/// Expects the encrypted block belongs to our key.
	///
	/// Errors:
	/// - [`StorageError::InvalidArgument`]: Block can not be decrypted.
	pub async fn set_encrypted(&self, block: Block<S::StoreParams>) -> Result<Cid, StorageError> {
		if MultiCodec::is(block.cid(), KnownMultiCodec::CoEncryptedBlock) {
			// decrypt the block to update the mapping
			let plain = EncryptedBlock::try_from(block.clone())
				.map_err(|e| StorageError::InvalidArgument(e.into()))?
				.block(&self.key)
				.map_err(|e| StorageError::InvalidArgument(e.into()))?;

			// write
			let encrypted_cid = self.next.set(block).await?;

			// map
			{
				self.mapping
					.extend([(*plain.cid(), encrypted_cid)].into_iter().chain(plain.references.clone()))
					.await;
			}

			// result
			Ok(encrypted_cid)
		} else {
			self.next.set(block).await
		}
	}
}
#[async_trait]
impl<S> BlockStorage for EncryptedBlockStorage<S>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
{
	type StoreParams = S::StoreParams;

	/// Get block.
	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		let encrypted_cid = self.mapping.get(cid).await;
		tracing::trace!(?encrypted_cid, ?cid, "encrypted-storage-get");
		match encrypted_cid {
			Some(encrypted_cid) => self.get_unencrypted(&encrypted_cid).await,
			None => self.next.get(cid).await,
		}
	}

	#[tracing::instrument(err, skip(self, block), fields(cid = ?block.cid()))]
	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		let cid = *block.cid();

		// references
		//  try to resolve all children references using the mapping
		//   the node creator has either:
		//    the mapping from loading the original node before
		//    all children nodes as he created it from sratch
		//  as a fallback check if the Cid exists in the parent so we know this is a unencrypted reference
		//  TODO: are there any valid cases for not unencrypted and not known reference?
		let references = if self.links.has_links(&cid) {
			let mut references = BTreeMap::new();
			for (plain_cid, encrypted_cid) in self.mapping.get_mapping(self.links.links(&block)?).await {
				match encrypted_cid {
					// reference mapping
					Some(encrypted_cid) => {
						references.insert(plain_cid, encrypted_cid);
					},
					// reference mode
					None => {
						if !self.reference_mode.is_reference_allowed(&self.next, plain_cid, cid).await {
							return Err(StorageError::InvalidArgument(anyhow!("Unmapped reference found {} while storing {}. Are you sure you stored all children nodes?", plain_cid, cid)));
						}
					},
				}
			}
			references
		} else {
			Default::default()
		};

		// encrypt
		let mut block: BlockPayload = block.into();
		block.references = references;
		let mut encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|e| StorageError::Internal(e.into()))?;

		// fit into block size limit
		let extra_encrypted_blocks = encrypted
			.payload
			.fit_into_blocks::<Self::StoreParams>(Some(Header::encoded_size(encrypted.header.algorithm)));
		let encrypted_block: Block<Self::StoreParams> = encrypted
			.try_into()
			.map_err(|e: AlgorithmError| StorageError::Internal(e.into()))?;

		// store
		for extra_encrypted_block in extra_encrypted_blocks {
			self.next.set(extra_encrypted_block).await?;
		}
		let encrypted_cid = self.next.set(encrypted_block).await?;

		// map
		self.mapping.insert(cid, encrypted_cid).await;

		// trace (only in debug because this has security implications)
		#[cfg(debug_assertions)]
		tracing::trace!(?cid, ?encrypted_cid, "storage-set");

		// result
		Ok(cid)
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		match self.mapping.get(cid).await {
			Some(encrypted_cid) => self.next.remove(&encrypted_cid).await,
			None => self.next.remove(cid).await,
		}
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.get(cid).await.map(|v| BlockStat { size: v.data().len() as u64 })
	}
}
impl<S> CloneWithBlockStorageSettings for EncryptedBlockStorage<S>
where
	S: CloneWithBlockStorageSettings,
{
	fn clone_with_settings(&self, settings: BlockStorageSettings) -> Self {
		EncryptedBlockStorage {
			key: self.key.clone(),
			algorithm: self.algorithm.clone(),
			links: self.links.clone(),
			reference_mode: self.reference_mode.clone(),
			mapping: if settings.detached { self.mapping.child() } else { self.mapping.clone() },
			next: self.next.clone_with_settings(settings),
		}
	}
}

#[derive(Debug, Default, Clone)]
pub enum EncryptionReferenceMode {
	/// Disallow any references that are not encrypted.
	/// Allowed references:
	/// - Plain: NO
	/// - Unrelated encrypted: YES
	#[default]
	DisallowPlain,

	/// Disallow any references that are not encrypted except specific plain references.
	/// Unrelated encrypted references are allowed.
	/// Allowed references:
	/// - Plain: SPECIFIC
	/// - Unrelated encrypted: YES
	DisallowPlainExcept(BTreeSet<Cid>),

	/// Disallow any references that are not encrypted except specific references.
	/// Allowed references:
	/// - Plain: SPECIFIC
	/// - Unrelated encrypted: SPECIFIC
	DisallowExcept(BTreeSet<Cid>),

	/// Allow any plain references.
	/// Allowed references:
	/// - Plain: YES
	/// - Unrelated encrypted: YES
	AllowPlain,

	/// Allow any plain references if the exists in parent storage.
	/// Allowed references:
	/// - Plain: IF EXISTS
	/// - Unrelated encrypted: YES
	AllowPlainIfExists,

	/// Allow any plain references but warn (log) about unencrypted references.
	/// Allowed references:
	/// - Plain: YES, WITH WARNING
	/// - Unrelated encrypted: YES
	Warning,
}
impl EncryptionReferenceMode {
	pub async fn is_reference_allowed<S>(&self, next: &S, reference: Cid, parent: Cid) -> bool
	where
		S: BlockStorage,
	{
		// encrypted block reference in plain data
		let is_unreleated_encrypted = KnownMultiCodec::CoEncryptedBlock == reference.codec();

		// evaluate
		match &self {
			EncryptionReferenceMode::DisallowPlain => is_unreleated_encrypted,
			EncryptionReferenceMode::DisallowPlainExcept(allowed) => {
				is_unreleated_encrypted || allowed.contains(&reference)
			},
			EncryptionReferenceMode::DisallowExcept(allowed) => allowed.contains(&reference),
			EncryptionReferenceMode::AllowPlain => true,
			EncryptionReferenceMode::AllowPlainIfExists => {
				is_unreleated_encrypted || next.stat(&reference).await.is_ok()
			},
			EncryptionReferenceMode::Warning => {
				tracing::warn!(mapped_cid = ?reference, cid = ?parent, "encrypted-storage-unmapped-reference");
				true
			},
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct EncryptedBlockStorageMapping {
	parent: Option<Arc<RwLock<BlockMapping>>>,
	mapping: Arc<RwLock<BlockMapping>>,
}
impl EncryptedBlockStorageMapping {
	/// Create a child instance.
	fn child(&self) -> EncryptedBlockStorageMapping {
		EncryptedBlockStorageMapping { parent: Some(self.mapping.clone()), mapping: Default::default() }
	}

	/// Load mapping from CID.
	/// This will add the mappings to the existing.
	pub async fn load_mapping<S>(&self, storage: &EncryptedBlockStorage<S>, map: &Cid) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + Send + Sync + 'static,
	{
		self.mapping.write().await.read_mappings(&storage, map).await?;
		Ok(())
	}

	/// Replace the mapping.
	pub async fn set_mapping(&mut self, mapping: BlockMapping) {
		self.parent = None;
		self.mapping = Arc::new(RwLock::new(mapping));
	}

	pub async fn get(&self, key: &Cid) -> Option<Cid> {
		match self.mapping.read().await.get(key) {
			Some(cid) => Some(cid),
			None => {
				if let Some(parent) = &self.parent {
					parent.read().await.get(key)
				} else {
					None
				}
			},
		}
	}

	pub async fn get_mapping(&self, keys: impl IntoIterator<Item = Cid>) -> BTreeMap<Cid, Option<Cid>> {
		let mapping = self.mapping.read().await;
		let parent = if let Some(parent) = &self.parent { Some(parent.read().await) } else { None };
		keys.into_iter()
			.map(|key| {
				let value = match mapping.get(&key) {
					Some(cid) => Some(cid),
					None => {
						if let Some(parent) = &parent {
							parent.get(&key)
						} else {
							None
						}
					},
				};
				(key, value)
			})
			.collect()
	}

	pub async fn get_first_by_value(&self, key: &Cid) -> Option<Cid> {
		match self.mapping.read().await.get_first_by_value(key) {
			Some(cid) => Some(cid),
			None => {
				if let Some(parent) = &self.parent {
					parent.read().await.get_first_by_value(key)
				} else {
					None
				}
			},
		}
	}

	pub async fn insert(&self, key: Cid, value: Cid) -> Option<Cid> {
		self.mapping.write().await.insert(key, value)
	}

	pub async fn extend(&self, items: impl IntoIterator<Item = (Cid, Cid)>) {
		self.mapping.write().await.extend(items);
	}

	pub async fn to_blocks<S, P: StoreParams>(
		&self,
		serializer: S,
		options: WriteOptions,
	) -> Result<(Option<Cid>, Vec<Block<P>>), StorageError>
	where
		S: NodeSerializer<Node<(Cid, Cid)>, (Cid, Cid), P>,
	{
		// copy items
		let mapping = {
			let mut map = self.mapping.read().await.map.clone();
			if let Some(parent) = &self.parent {
				let mut parent_map = parent.read().await.map.clone();
				map.append(&mut parent_map);
			};
			let mut result = BlockMapping::new();
			result.map = map;
			result
		};

		// blocks
		mapping.to_blocks(serializer, options)
	}
}
#[async_trait]
impl BlockStorageContentMapping for EncryptedBlockStorageMapping {
	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.get(mapped).await
	}

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.get_first_by_value(plain).await
	}
}

#[derive(Debug, thiserror::Error)]
pub enum BlockMappingError {
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
#[derive(Debug)]
pub struct BlockMapping {
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

	pub fn extend(&mut self, items: impl IntoIterator<Item = (Cid, Cid)>) {
		self.map.extend(items.into_iter().map(|(key, value)| (key, value)));
	}

	pub fn get_first_by_value(&self, value: &Cid) -> Option<Cid> {
		self.map.iter().find_map(|(k, v)| {
			if v == value {
				return Some(*k);
			}
			None
		})
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
				let encrypted_block: EncryptedBlock = storage.next.get(&cid).await?.try_into()?;
				let block = encrypted_block.block(&storage.key)?;
				mapping.insert(block.cid, cid);
			}
		}
		Ok(mapping)
	}

	/// Read block mappings from `cid` via an block storage.
	/// Idempotency: Yes
	pub async fn read_mappings<S>(
		&mut self,
		storage: &EncryptedBlockStorage<S>,
		cid: &Cid,
	) -> Result<usize, StorageError>
	where
		S: BlockStorage + Clone + Send + Sync + 'static,
	{
		let mut count = 0;
		let mut tasks = FuturesOrdered::new();

		// first
		let read = |cid: Cid| async move { storage.get_unencrypted(&cid).await };
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
	pub fn to_blocks<S, P: StoreParams>(
		&self,
		serializer: S,
		options: WriteOptions,
	) -> Result<(Option<Cid>, Vec<Block<P>>), StorageError>
	where
		S: NodeSerializer<Node<(Cid, Cid)>, (Cid, Cid), P>,
	{
		// blocks
		let mut builder = NodeBuilder::<(Cid, Cid), P, Node<(Cid, Cid)>, S>::new(options.max_children, serializer);
		for (key, value) in self.map.iter() {
			builder.push((*key, *value)).map_err(|e| StorageError::Internal(e.into()))?;
		}
		let (root, blocks) = builder.into_blocks().map_err(|e| StorageError::Internal(e.into()))?;

		// result
		Ok((*root.cid(), blocks))
	}
}
impl Default for BlockMapping {
	fn default() -> Self {
		Self::new()
	}
}

/// Create encrypted block.
struct EncryptedNodeSerializer {
	key: Secret,
	algorithm: Algorithm,
}
impl<T, P> NodeSerializer<Node<T>, T, P> for EncryptedNodeSerializer
where
	T: Clone + Serialize,
	P: StoreParams,
{
	fn nodes(&mut self, nodes: Vec<Link<Node<T>>>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Node(nodes))
	}

	fn leaf(&mut self, entries: Vec<T>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Leaf(entries))
	}

	fn serialize(&mut self, node: Node<T>) -> Result<Block<P>, NodeBuilderError> {
		let block: Block<P> = DefaultNodeSerializer::new().serialize(node)?;
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

pub struct WriteOptions {
	/// Max byte size for each block.
	// max_size: usize,

	/// Max children for each block.
	max_children: usize,
}
impl Default for WriteOptions {
	fn default() -> Self {
		Self {
			// max_size: 2.pow(18),
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
		BlockStorage, EncryptedBlockStorage, MemoryBlockStorage,
	};
	use cid::Cid;
	use co_primitives::{BlockSerializer, DefaultParams, StorageError, StoreParams};
	use serde::{Deserialize, Serialize};
	use std::iter::repeat;

	#[derive(Debug, Serialize, Deserialize)]
	struct Test {
		hello: String,
	}

	#[tokio::test]
	async fn roundtrip() {
		// storage
		let memory = MemoryBlockStorage::default();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let encryption = EncryptedBlockStorage::new(memory.clone(), key, algorithm, Default::default());

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

	#[tokio::test]
	async fn store_mapping() {
		// storage
		let memory = MemoryBlockStorage::new();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
		let encryption = EncryptedBlockStorage::new(memory.clone(), key.clone(), algorithm, Default::default());

		// blocks
		let mut cids: Vec<Cid> = Default::default();
		for i in 0..1024 {
			let data = Test { hello: format!("Hi {}!", i).to_owned() };
			let block = BlockSerializer::default().serialize(&data).unwrap();
			cids.push(*block.cid());
			encryption.set(block.clone()).await.unwrap();
		}

		// validate mapping
		let mapping_cid = encryption.flush_mapping().await.unwrap().expect("Mappings if we have items");
		assert_eq!(mapping_cid.codec(), BLOCK_MULTICODEC); // encrypted?

		// validate cids
		let memory_cids: Vec<Cid> = memory.entries().await.map(|block| *block.cid()).collect();
		assert_eq!(memory_cids.len(), 7 + 1024); // 7 (merkle) mapping blocks and 1024 data blocks
		for memory_cid in memory_cids.iter() {
			let memory_block = memory.get(memory_cid).await.unwrap();
			assert_eq!(memory_cid.codec(), BLOCK_MULTICODEC); // all blocks are encrypted
			assert!(DefaultParams::MAX_BLOCK_SIZE > memory_block.data().len()); // all blocks fit in max block size
		}

		// validate load blocks again
		let encryption = EncryptedBlockStorage::new(memory, key, algorithm, Default::default());
		encryption.load_mapping(&mapping_cid).await.unwrap();
		for cid in cids {
			encryption.get(&cid).await.unwrap();
		}
	}
}
