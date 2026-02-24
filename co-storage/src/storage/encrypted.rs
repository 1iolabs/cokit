// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	crypto::{
		block::{Algorithm, BlockPayload, EncryptedBlock, Header, BLOCK_MULTICODEC},
		secret::Secret,
	},
	AlgorithmError, BlockStorageContentMapping, ExtendedBlock, ExtendedBlockStorage, StorageError,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_primitives::{
	from_cbor, AnyBlockStorage, Block, BlockLinks, BlockStat, BlockStorage, BlockStorageCloneSettings,
	CloneWithBlockStorageSettings, DefaultNodeSerializer, KnownMultiCodec, Link, MappedCid, MultiCodec, Node,
	NodeBuilder, NodeBuilderError, NodeSerializer,
};
use futures::{
	stream::{self, FuturesOrdered},
	StreamExt, TryStreamExt,
};
use serde::Serialize;
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub struct EncryptedBlockStorage<S> {
	key: Secret,
	algorithm: Algorithm,
	next: S,
	mapping: EncryptedBlockStorageMapping,
	links: BlockLinks,
	reference_mode: EncryptionReferenceMode,
	transform: bool,
}
impl<S> EncryptedBlockStorage<S>
where
	S: AnyBlockStorage,
{
	pub fn new(next: S, key: Secret, algorithm: Algorithm, mapping: EncryptedBlockStorageMapping) -> Self {
		Self {
			algorithm,
			key,
			mapping,
			next,
			links: Default::default(),
			reference_mode: Default::default(),
			transform: false,
		}
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

	/// Add known mappings.
	pub async fn insert_mappings(&self, mappings: impl IntoIterator<Item = MappedCid>) {
		self.mapping.extend(mappings).await;
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
		let (root, blocks) = self
			.mapping
			.to_blocks(node_serializer, WriteOptions::new(self.max_block_size()))
			.await?;

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

	/// Clear encryption mapping.
	pub async fn clear_mapping(&self, keep: impl IntoIterator<Item = Cid>) {
		let mapping = self.mapping.get_mapping(keep).await;

		// clear
		self.mapping.clear().await;

		// add
		self.mapping
			.extend(
				mapping
					.into_iter()
					.filter_map(|(key, value)| value.map(|value| MappedCid(key, value))),
			)
			.await;
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
	pub async fn get_unencrypted(&self, cid: &Cid) -> Result<Block, StorageError> {
		Ok(if MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			// get block
			let mut block =
				EncryptedBlock::try_from(self.next.get(cid).await?).map_err(|e| StorageError::Internal(e.into()))?;

			// make inline
			if let Some(blocks) = block.payload.blocks() {
				let blocks = stream::iter(blocks.iter().cloned())
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
			self.mapping
				.extend(
					[(plain.cid, *cid)]
						.into_iter()
						.chain(plain.references.clone().into_iter())
						.map(|(internal, external)| MappedCid::new(internal, external)),
				)
				.await;

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
	pub async fn set_encrypted(&self, block: Block) -> Result<Cid, StorageError> {
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
					.extend(
						[(*plain.cid(), encrypted_cid)]
							.into_iter()
							.chain(plain.references.clone())
							.map(|(internal, external)| MappedCid::new(internal, external)),
					)
					.await;
			}

			// result
			Ok(encrypted_cid)
		} else {
			self.next.set(block).await
		}
	}

	#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(self, extended_block), fields(cid = ?extended_block.block.cid()))]
	async fn set_block(&self, extended_block: ExtendedBlock) -> Result<Cid, StorageError> {
		let block = extended_block.block;
		let cid = *block.cid();

		// log
		#[cfg(feature = "logging-verbose")]
		{
			if co_primitives::MultiCodec::is_cbor(block.cid()) {
				tracing::trace!(cid = ?block.cid(), ipld = ?from_cbor::<ipld_core::ipld::Ipld>(block.data()), "set");
			} else {
				tracing::trace!(cid = ?block.cid(), "set");
			}
		}

		// references
		//  try to resolve all children references using the mapping
		//   the node creator has either:
		//    the mapping from loading the original node before
		//    all children nodes as he created it from sratch
		//  as a fallback check if the Cid exists in the parent so we know this is a unencrypted reference
		//  Question: are there any valid cases for not unencrypted and not known reference?
		// 	  Yes: encrypted local CO stores references to unencrypted shared CO.
		let mut references = extended_block.options.references.unwrap_or_default();
		if self.links.has_links(cid) {
			// links
			//  filter out already mapped links
			let links = self.links.links(&block)?.filter(|link| !references.contains_key(link));

			// references
			for (plain_cid, encrypted_cid) in self.mapping.get_mapping(links).await {
				match encrypted_cid {
					// reference mapping
					Some(encrypted_cid) => {
						references.insert(plain_cid, encrypted_cid);
					},
					// reference mode
					None => {
						// log
						if let EncryptionReferenceMode::Warning = self.reference_mode {
							tracing::trace!(unmapped_cid = ?plain_cid, ?cid, all_references = ?references, all_links = ?self.links.links(&block).map(|i| i.collect::<Vec<Cid>>()), "unmapped-reference");
						}

						// error
						if !self.reference_mode.is_reference_allowed(&self.next, plain_cid, cid).await {
							return Err(StorageError::InvalidArgument(anyhow!("Unmapped reference found {} while storing {}. Are you sure you stored all children nodes?", plain_cid, cid)));
						}
					},
				}
			}
		}

		// encrypt
		let mut block: BlockPayload = block.into();
		block.references = references;
		let mut encrypted =
			EncryptedBlock::encrypt(self.algorithm, &self.key, block).map_err(|e| StorageError::Internal(e.into()))?;

		// fit into block size limit
		let extra_encrypted_blocks = encrypted
			.payload
			.fit_into_blocks(self.next.max_block_size(), Some(Header::encoded_size(encrypted.header.algorithm)));
		let encrypted_block: Block = encrypted
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
		tracing::trace!(?cid, ?encrypted_cid, cid_bytes = ?cid.to_bytes(), encrypted_cid_bytes = ?encrypted_cid.to_bytes(), "storage-set");

		// result
		Ok(cid)
	}
}
#[async_trait]
impl<S> BlockStorage for EncryptedBlockStorage<S>
where
	S: AnyBlockStorage,
{
	/// Get block.
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		// transform?
		if self.transform && MultiCodec::is(cid, KnownMultiCodec::CoEncryptedBlock) {
			return self.get_unencrypted(cid).await;
		}

		// default
		let encrypted_cid = self.mapping.get(cid).await;
		#[cfg(feature = "logging-verbose")]
		tracing::trace!(?encrypted_cid, ?cid, "encrypted-storage-get");
		match encrypted_cid {
			Some(encrypted_cid) => self.get_unencrypted(&encrypted_cid).await,
			None => match self.next.get(cid).await {
				Err(err @ StorageError::NotFound(_, _)) => {
					// log
					#[cfg(feature = "logging-verbose")]
					{
						let mapping = self.mapping.mapping.read().unwrap().map.clone();
						let parent_mapping =
							self.mapping.parent.as_ref().map(|parent| parent.read().unwrap().map.clone());
						tracing::warn!(?mapping, ?parent_mapping, ?err, ?cid, "encrypted-storage-get-not-found");
					}

					// forward
					Err(err)
				},
				i => i,
			},
		}
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.set_block(block.into()).await
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		match self.mapping.remove(cid).await {
			Some(encrypted_cid) => {
				// log
				#[cfg(feature = "logging-verbose")]
				tracing::trace!(?encrypted_cid, ?cid, "encrypted-storage-remove");

				// remove
				self.next.remove(&encrypted_cid).await
			},
			None => self.next.remove(cid).await,
		}
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.get(cid).await.map(|v| BlockStat { size: v.data().len() as u64 })
	}

	fn max_block_size(&self) -> usize {
		self.next.max_block_size()
	}
}
#[async_trait]
impl<S> ExtendedBlockStorage for EncryptedBlockStorage<S>
where
	S: ExtendedBlockStorage + Clone + 'static,
{
	async fn set_extended(&self, extended_block: ExtendedBlock) -> Result<Cid, StorageError> {
		self.set_block(extended_block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		match self.mapping.get(cid).await {
			Some(encrypted_cid) => self.next.exists(&encrypted_cid).await,
			None => self.next.exists(cid).await,
		}
	}

	async fn clear(&self) -> Result<(), StorageError> {
		self.next.clear().await
	}
}
impl<S> CloneWithBlockStorageSettings for EncryptedBlockStorage<S>
where
	S: CloneWithBlockStorageSettings,
{
	fn clone_with_settings(&self, settings: BlockStorageCloneSettings) -> Self {
		EncryptedBlockStorage {
			key: self.key.clone(),
			algorithm: self.algorithm,
			links: self.links.clone(),
			reference_mode: self.reference_mode.clone(),
			mapping: if settings.clear {
				Default::default()
			} else if settings.detached {
				self.mapping.child()
			} else {
				self.mapping.clone()
			},
			transform: settings.transform,
			next: self.next.clone_with_settings(settings),
		}
	}
}
#[async_trait]
impl<S> BlockStorageContentMapping for EncryptedBlockStorage<S>
where
	S: BlockStorage + Clone + Send + Sync + 'static,
{
	async fn is_content_mapped(&self) -> bool {
		true
	}

	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.mapping.get(mapped).await
	}

	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		// try to read from mapping
		if let Some(mapped) = self.mapping.get_first_by_value(plain).await {
			return Some(mapped);
		}

		// try to decrypt
		if MultiCodec::is(plain, KnownMultiCodec::CoEncryptedBlock) {
			if let Ok(block) = self.get_unencrypted(plain).await {
				return Some(*block.cid());
			}
		}

		// none
		None
	}

	async fn insert_mappings(&self, mappings: BTreeSet<MappedCid>) {
		self.mapping.extend(mappings).await;
	}
}

#[derive(Debug, Default, Clone)]
pub enum EncryptionReferenceMode {
	/// Disallow any references that are not encrypted.
	/// Allowed references:
	/// - Plain: NO
	/// - Unrelated encrypted: YES
	/// - CoReference: YES
	#[default]
	DisallowPlain,

	/// Disallow any references that are not encrypted except specific plain references.
	/// Unrelated encrypted references are allowed.
	/// Allowed references:
	/// - Plain: SPECIFIC
	/// - Unrelated encrypted: YES
	/// - CoReference: YES
	DisallowPlainExcept(BTreeSet<Cid>),

	/// Disallow any references that are not encrypted except specific references.
	/// Allowed references:
	/// - Plain: SPECIFIC
	/// - Unrelated encrypted: SPECIFIC
	/// - CoReference: YES
	DisallowExcept(BTreeSet<Cid>),

	/// Allow any plain references.
	/// Allowed references:
	/// - Plain: YES
	/// - Unrelated encrypted: YES
	/// - CoReference: YES
	AllowPlain,

	/// Allow any plain references if the exists in parent storage.
	/// Allowed references:
	/// - Plain: IF EXISTS
	/// - Unrelated encrypted: YES
	/// - CoReference: YES
	AllowPlainIfExists,

	/// Allow any plain references but warn (log) about unencrypted references.
	/// Allowed references:
	/// - Plain: YES, WITH WARNING
	/// - Unrelated encrypted: YES
	/// - CoReference: YES
	Warning,
}
impl EncryptionReferenceMode {
	/// Test if reference is allowed in parent.
	///
	/// Note: For mode Warning, the caller is responsible for the warning.
	pub async fn is_reference_allowed<S>(&self, next: &S, reference: Cid, parent: Cid) -> bool
	where
		S: BlockStorage,
	{
		// encrypted block reference in plain data
		let is_unreleated_encrypted = MultiCodec::is(reference, KnownMultiCodec::CoEncryptedBlock);
		let is_co_reference = MultiCodec::is(parent, KnownMultiCodec::CoReference);

		// evaluate
		match &self {
			EncryptionReferenceMode::DisallowPlain => is_co_reference || is_unreleated_encrypted,
			EncryptionReferenceMode::DisallowPlainExcept(allowed) => {
				is_co_reference || is_unreleated_encrypted || allowed.contains(&reference)
			},
			EncryptionReferenceMode::DisallowExcept(allowed) => is_co_reference || allowed.contains(&reference),
			EncryptionReferenceMode::AllowPlain => true,
			EncryptionReferenceMode::AllowPlainIfExists => {
				is_co_reference || is_unreleated_encrypted || next.stat(&reference).await.is_ok()
			},
			EncryptionReferenceMode::Warning => true,
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct EncryptedBlockStorageMapping {
	mapping: Arc<RwLock<BlockMapping>>,

	/// Parent Mapping.
	///
	/// Note: To prevent deadlocks, when lock both always lock mapping first.
	parent: Option<Arc<RwLock<BlockMapping>>>,
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
		// load
		let mut mapping = BlockMapping::new();
		mapping.read_mappings(storage, map).await?;

		// insert
		//  we dont use read_mappings directly because of possible deadlocks and because it involves IO.
		self.mapping.write().unwrap().append(&mut mapping);

		// done
		Ok(())
	}

	/// Replace the mapping.
	pub async fn set_mapping(&mut self, mapping: BlockMapping) {
		self.parent = None;
		self.mapping = Arc::new(RwLock::new(mapping));
	}

	pub async fn clear(&self) {
		self.mapping.write().unwrap().clear();
	}

	pub async fn get(&self, key: &Cid) -> Option<Cid> {
		match self.mapping.read().unwrap().get(key) {
			Some(cid) => Some(cid),
			None => {
				if let Some(parent) = &self.parent {
					parent.read().unwrap().get(key)
				} else {
					None
				}
			},
		}
	}

	pub async fn remove(&self, key: &Cid) -> Option<Cid> {
		let mut result = self.mapping.write().unwrap().remove(key);
		if let Some(parent) = &self.parent {
			let parent_result = parent.write().unwrap().remove(key);
			if result.is_none() {
				result = parent_result;
			}
		}
		result
	}

	/// Map multiple Cids into an Map.
	pub async fn get_mapping(&self, keys: impl IntoIterator<Item = Cid>) -> BTreeMap<Cid, Option<Cid>> {
		let mapping = self.mapping.read().unwrap();
		let parent = self.parent.as_ref().map(|parent| parent.read().unwrap());
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
		match self.mapping.read().unwrap().get_first_by_value(key) {
			Some(cid) => Some(cid),
			None => {
				if let Some(parent) = &self.parent {
					parent.read().unwrap().get_first_by_value(key)
				} else {
					None
				}
			},
		}
	}

	pub async fn insert(&self, key: Cid, value: Cid) -> Option<Cid> {
		self.mapping.write().unwrap().insert(key, value)
	}

	pub async fn extend(&self, items: impl IntoIterator<Item = MappedCid>) {
		self.mapping
			.write()
			.unwrap()
			.extend(items.into_iter().map(|MappedCid(internal, external)| (internal, external)));
	}

	pub async fn to_blocks<S>(
		&self,
		serializer: S,
		options: WriteOptions,
	) -> Result<(Option<Cid>, Vec<Block>), StorageError>
	where
		S: NodeSerializer<Node<(Cid, Cid)>, (Cid, Cid)>,
	{
		// copy items
		let mapping = {
			let mut map = self.mapping.read().unwrap().clone();
			if let Some(parent) = &self.parent {
				let mut parent_map = parent.read().unwrap().clone();
				map.append(&mut parent_map);
			};
			map
		};

		// blocks
		mapping.to_blocks(serializer, options)
	}
}
#[async_trait]
impl BlockStorageContentMapping for EncryptedBlockStorageMapping {
	async fn is_content_mapped(&self) -> bool {
		true
	}

	/// Convert the mapped [`Cid`] to an plain storage [`Cid`].
	async fn to_plain(&self, mapped: &Cid) -> Option<Cid> {
		self.get(mapped).await
	}

	/// Convert the plain storage [`Cid`] to a mapped [`Cid`].
	async fn to_mapped(&self, plain: &Cid) -> Option<Cid> {
		self.get_first_by_value(plain).await
	}

	async fn insert_mappings(&self, _mappings: BTreeSet<MappedCid>) {
		unimplemented!("use storage directly");
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
#[derive(Clone, Debug)]
pub struct BlockMapping {
	/// Mapping from mapped/internal to plain/external.
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
		self.map.extend(items);
	}

	pub fn append(&mut self, other: &mut BlockMapping) {
		self.map.append(&mut other.map);
	}

	pub fn remove(&mut self, key: &Cid) -> Option<Cid> {
		self.map.remove(key)
	}

	pub fn get_first_by_value(&self, value: &Cid) -> Option<Cid> {
		self.map.iter().find_map(|(k, v)| {
			if v == value {
				return Some(*k);
			}
			None
		})
	}

	pub fn clear(&mut self) {
		self.map.clear();
	}

	pub fn iter(&self) -> impl Iterator<Item = (&Cid, &Cid)> {
		self.map.iter()
	}

	pub fn into_iter(self) -> impl Iterator<Item = (Cid, Cid)> {
		self.map.into_iter()
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
			MultiCodec::with_cbor(block.cid())?;

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
	pub fn to_blocks<S>(&self, serializer: S, options: WriteOptions) -> Result<(Option<Cid>, Vec<Block>), StorageError>
	where
		S: NodeSerializer<Node<(Cid, Cid)>, (Cid, Cid)>,
	{
		// blocks
		let mut builder = NodeBuilder::<(Cid, Cid), Node<(Cid, Cid)>, S>::new(
			options.max_block_size,
			options.max_children,
			serializer,
		);
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
impl<T> NodeSerializer<Node<T>, T> for EncryptedNodeSerializer
where
	T: Clone + Serialize,
{
	fn nodes(&mut self, nodes: Vec<Link<Node<T>>>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Node(nodes))
	}

	fn leaf(&mut self, entries: Vec<T>) -> Result<Node<T>, NodeBuilderError> {
		Ok(Node::Leaf(entries))
	}

	fn serialize(&mut self, max_block_size: usize, node: Node<T>) -> Result<Block, NodeBuilderError> {
		let block: Block = DefaultNodeSerializer::new().serialize(max_block_size, node)?;
		let encrypted = EncryptedBlock::encrypt(self.algorithm, &self.key, block)?;
		let encrypted_block: Block = encrypted.try_into()?;
		Ok(encrypted_block)
	}
}
impl From<AlgorithmError> for NodeBuilderError {
	fn from(err: AlgorithmError) -> Self {
		NodeBuilderError::Encoding(err.into())
	}
}

pub struct WriteOptions {
	/// Max byte size for each block.
	max_block_size: usize,

	/// Max children for each block.
	max_children: usize,
}
impl WriteOptions {
	fn new(max_block_size: usize) -> Self {
		Self { max_block_size, max_children: 174 }
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
	use std::iter::repeat_n;

	#[derive(Debug, Serialize, Deserialize)]
	struct Test {
		hello: String,
	}

	#[tokio::test]
	async fn roundtrip() {
		// storage
		let memory = MemoryBlockStorage::default();
		let algorithm = Algorithm::default();
		let key = Secret::new(repeat_n(42, algorithm.key_size()).collect());
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
		let key = Secret::new(repeat_n(42, algorithm.key_size()).collect());
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
