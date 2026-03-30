// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use anyhow::anyhow;
use co_api::{
	co, BlockStorage, BlockStorageExt, CoList, CoListTransaction, CoMap, CoMapTransaction, CoSet, CoreBlockStorage,
	IsDefault, LazyTransaction, Link, OptionLink, Reducer, ReducerAction, StorageError, Tags, WeakCid,
};
use futures::{pin_mut, FutureExt, TryStreamExt};
use std::collections::{BTreeMap, BTreeSet};

#[co(state)]
pub struct Storage {
	/// Named pins.
	#[serde(rename = "p", default, skip_serializing_if = "CoMap::is_empty")]
	pub pins: CoMap<String, Pin>,

	/// Block metadata.
	#[serde(rename = "b", default, skip_serializing_if = "CoMap::is_empty")]
	pub blocks: CoMap<WeakCid, BlockMetadata>,

	/// Block metadata index to unreferenced (reference count of zero and children resolved) entries.
	/// See: [`BlockMetadata::is_removable`]
	#[serde(rename = "bu", default, skip_serializing_if = "CoMap::is_empty")]
	pub blocks_index_unreferenced: CoMap<WeakCid, BlockInfo>,

	/// Blocks that are recursively added but children are pending.
	/// Blocks that are recursively deleted but children has not yet unreferenced.
	#[serde(rename = "bs", default, skip_serializing_if = "CoMap::is_empty")]
	pub block_structure_pending: CoMap<WeakCid, BlockStructurePending>,
}

#[co]
pub enum BlockStructurePending {
	/// recursively added but children has not yet referenced.
	Reference(BlockInfo),
}
impl BlockStructurePending {
	pub fn info(&self) -> &BlockInfo {
		match self {
			BlockStructurePending::Reference(block_info) => block_info,
		}
	}
}

#[co]
#[derive(Default)]
pub enum ReferenceMode {
	/// Reference is shallow. Children not yet referenced.
	#[default]
	#[serde(rename = "s")]
	Shallow,

	/// All direct children has been referenced by this reference.
	#[serde(rename = "r")]
	Recursive,
}
impl ReferenceMode {
	pub fn is_recursive(&self) -> bool {
		match self {
			ReferenceMode::Shallow => false,
			ReferenceMode::Recursive => true,
		}
	}
}

#[co]
pub struct BlockInfo {
	/// Pinning keys that reference this block.
	#[serde(rename = "p", default, skip_serializing_if = "CoSet::is_empty")]
	pub pins: CoSet<String>,

	/// This is a root reference.
	#[serde(rename = "t", default, skip_serializing_if = "BlockType::is_unknown")]
	pub block_type: BlockType,
}
impl BlockInfo {
	pub async fn new<S>(storage: &S, pin: String, block_type: BlockType) -> Result<Self, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut pins = CoSet::default();
		pins.insert(storage, pin).await?;
		Ok(Self { pins, block_type })
	}

	pub fn with_block_type(mut self, block_type: BlockType) -> Self {
		self.block_type = block_type;
		self
	}
}

#[co]
#[derive(Default)]
pub enum BlockType {
	#[default]
	Unknown,

	/// Block type will be set to root if is caused by a pin operation (create/remove).
	Root,
}
impl BlockType {
	pub fn is_unknown(&self) -> bool {
		matches!(self, BlockType::Unknown)
	}

	pub fn is_root(&self) -> bool {
		matches!(self, BlockType::Root)
	}
}

#[co]
#[derive(Default)]
pub struct BlockMetadata {
	/// Current reference count on this node.
	#[serde(rename = "r")]
	pub references: u32,

	/// Reference mode.
	#[serde(rename = "m", default, skip_serializing_if = "IsDefault::is_default")]
	pub mode: ReferenceMode,

	/// Additional metadata.
	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}
impl BlockMetadata {
	pub fn is_removable(&self) -> bool {
		self.references == 0 && self.mode.is_recursive()
	}
}

#[co]
#[derive(Default)]
pub struct Pin {
	/// Free strategy.
	#[serde(rename = "s")]
	pub strategy: PinStrategy,

	/// Pinned references.
	/// Sorted by insertion (oldest is first).
	/// Every pinned item will automatically maintain a reference count.
	#[serde(rename = "r", default, skip_serializing_if = "CoList::is_empty")]
	pub references: CoList<WeakCid>,

	/// Pinned references count.
	#[serde(rename = "c")]
	pub references_count: u32,
}

#[co]
#[derive(Default)]
pub enum PinStrategy {
	/// Unlimited pins.
	#[default]
	#[serde(rename = "u")]
	Unlimited,

	/// Maximum count of references.
	#[serde(rename = "h")]
	MaxCount(u32),
}

/// A list of references.
/// A single [`Cid`] is allowed to be contained multiple times (=reference count).
#[co]
#[derive(Default)]
#[serde(transparent)]
pub struct References(Vec<(WeakCid, BlockMetadata)>);
impl References {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(&mut self, reference: impl Into<WeakCid>) {
		let reference = reference.into();
		let (_, block) = match self.0.iter_mut().find(|(cid, _block)| *cid == reference) {
			Some(block) => block,
			None => {
				self.0.push((reference, Default::default()));
				// SAFETY: Just created.
				self.0.last_mut().unwrap()
			},
		};
		block.references += 1;
	}

	pub fn insert_with_tags(&mut self, reference: impl Into<WeakCid>, tags: Tags) {
		let reference = reference.into();
		let (_, block) = match self.0.iter_mut().find(|(cid, _block)| *cid == reference) {
			Some(block) => block,
			None => {
				self.0.push((reference, Default::default()));
				// SAFETY: Just created.
				self.0.last_mut().unwrap()
			},
		};
		block.references += 1;
		block.tags.extend(tags);
	}

	pub fn extend(&mut self, references: impl IntoIterator<Item = WeakCid>) {
		for item in references.into_iter() {
			self.insert(item);
		}
	}

	pub fn extend_with_tags(&mut self, references: impl IntoIterator<Item = WeakCid>, tags: Tags) {
		for item in references.into_iter() {
			self.insert_with_tags(item, tags.clone());
		}
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn iter(&self) -> impl Iterator<Item = WeakCid> + use<'_> {
		self.0.iter().map(|(cid, _)| *cid)
	}

	pub fn iter_with_tags(&self) -> impl Iterator<Item = (WeakCid, &'_ Tags)> + use<'_> {
		self.0.iter().map(|(cid, tags)| (*cid, &tags.tags))
	}
}
impl FromIterator<WeakCid> for References {
	fn from_iter<T: IntoIterator<Item = WeakCid>>(iter: T) -> Self {
		let mut result = Self::default();
		result.extend(iter);
		result
	}
}
impl<const N: usize> From<[WeakCid; N]> for References {
	fn from(arr: [WeakCid; N]) -> Self {
		if N == 0 {
			return Self::default();
		}
		Self::from_iter(arr)
	}
}

#[co]
pub enum StorageAction {
	/// Increase [`Cid`] reference count by one.
	/// Refernces are creates on-the-fly if not exist.
	/// Shallow: [`Cid`] links are not added automatically (not recusrive).
	///
	/// # Args
	/// - BlockInfo contains causality data about the reference
	/// - The blocks to reference
	/// - Tags will be merged with [`BlockMetadata::tags`]
	#[serde(rename = "r")]
	Reference(BlockInfo, References),

	/// Decrease [`Cid`] reference count by one.
	#[serde(rename = "u")]
	Unreference(BlockInfo, References),

	/// Structurally reference/delete [`Cid`].
	/// Expects all children references passed for a parent even is they not exist on disk.
	///
	/// # Vec Arguments
	/// - `0`: The parent reference.
	/// - `1`: The links of the parent reference.
	#[serde(rename = "s")]
	Structure(#[serde(with = "co_api::serde_map_as_list")] BTreeMap<WeakCid, References>),

	/// Create [`Cid`] references with ref count of zero if the reference not exists yet.
	/// This is normally used to track newly created blocks.
	///
	/// # Arguments
	/// - `0`: The [`Cid`] of entries to create.
	#[serde(rename = "c")]
	ReferenceCreate(BlockInfo, References),

	/// Mark to remove [`Cid`]. This will make the references shallow again.
	/// And eventually shedule them to delete.
	///
	/// # Note
	/// This is basically the same as Unreference.
	///
	/// # Arguments
	/// - `0`: The BlockInfo of the blocks to remove.
	/// - `0`: The [`Cid`] of entries to remove.
	/// - `1`: Force remove all instances.
	#[serde(rename = "ud")]
	Remove(BlockInfo, BTreeSet<WeakCid>, bool),

	/// Delete [`Cid`] references.
	///
	/// # Arguments
	/// - `0`: The [`Cid`] of entries to remove with all of its direct children.
	/// - `1`: Force delete. If false only references with a zero ref count will be removed.
	#[serde(rename = "d")]
	Delete(BlockInfo, BTreeMap<WeakCid, BTreeSet<WeakCid>>, bool),

	/// Append tags to references.
	#[serde(rename = "ti")]
	TagsInsert(BTreeSet<WeakCid>, Tags),

	/// Remove tags from references.
	#[serde(rename = "tr")]
	TagsRemove(BTreeSet<WeakCid>, Tags),

	/// Create a named pin and reference all specified [`Cid`]s.
	#[serde(rename = "pc")]
	PinCreate(String, PinStrategy, References),

	/// Update a named pin by setting the [`PinStrategy`].
	#[serde(rename = "pu")]
	PinUpdate(String, PinStrategy),

	/// Insert references to a named pin and reference all specified [`Cid`]s.
	#[serde(rename = "pr")]
	PinReference(String, References),

	/// Remove a named pin and unreference all [`Cid`]s.
	#[serde(rename = "pd")]
	PinRemove(String),

	/// Batch process actions.
	#[serde(rename = "b")]
	Batch(CoList<StorageAction>),
}
impl Storage {
	/// Create inital state.
	pub async fn initial_state<S: BlockStorage + Clone + 'static>(
		storage: &S,
		actions: Vec<StorageAction>,
	) -> Result<OptionLink<Self>, anyhow::Error> {
		let mut state = Storage::default();
		let mut transaction = StorageTransaction::open(storage.clone(), &state).await?;
		for action in actions {
			reduce(&mut transaction, action).await?;
		}
		transaction.store(&mut state).await?;
		Ok(storage.set_value(&state).await?.into())
	}
}
impl Reducer<StorageAction> for Storage {
	async fn reduce(
		state: OptionLink<Self>,
		event: Link<ReducerAction<StorageAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event).await?;
		let mut state = storage.get_value_or_default(&state).await?;
		let mut transaction = StorageTransaction::open(storage.clone(), &state).await?;
		reduce(&mut transaction, event.payload).await?;
		transaction.store(&mut state).await?;
		Ok(storage.set_value(&state).await?)
	}
}

struct StorageTransaction<S>
where
	S: BlockStorage + Clone + 'static,
{
	storage: S,
	pins_changed: bool,
	pins: LazyTransaction<S, CoMap<String, Pin>>,
	blocks_changed: bool,
	blocks: LazyTransaction<S, CoMap<WeakCid, BlockMetadata>>,
	blocks_index_unreferenced_changed: bool,
	blocks_index_unreferenced: LazyTransaction<S, CoMap<WeakCid, BlockInfo>>,
	block_structure_pending_changed: bool,
	block_structure_pending: LazyTransaction<S, CoMap<WeakCid, BlockStructurePending>>,
}
impl<S> StorageTransaction<S>
where
	S: BlockStorage + Clone + 'static,
{
	async fn open(storage: S, state: &Storage) -> Result<Self, anyhow::Error> {
		Ok(Self {
			pins_changed: false,
			pins: state.pins.open_lazy(&storage).await?,
			blocks_changed: false,
			blocks: state.blocks.open_lazy(&storage).await?,
			blocks_index_unreferenced_changed: false,
			blocks_index_unreferenced: state.blocks_index_unreferenced.open_lazy(&storage).await?,
			block_structure_pending_changed: false,
			block_structure_pending: state.block_structure_pending.open_lazy(&storage).await?,
			storage,
		})
	}

	async fn store(&mut self, state: &mut Storage) -> Result<(), anyhow::Error> {
		if let Some(pins) = self.pins.opt_mut() {
			if self.pins_changed {
				state.pins = pins.store().await?;
				self.pins_changed = false;
			}
		}
		if let Some(blocks) = self.blocks.opt_mut() {
			if self.blocks_changed {
				state.blocks = blocks.store().await?;
				self.blocks_changed = false;
			}
		}
		if let Some(blocks_index_unreferenced) = self.blocks_index_unreferenced.opt_mut() {
			if self.blocks_index_unreferenced_changed {
				state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;
				self.blocks_index_unreferenced_changed = false;
			}
		}
		if let Some(block_structure_pending) = self.block_structure_pending.opt_mut() {
			if self.block_structure_pending_changed {
				state.block_structure_pending = block_structure_pending.store().await?;
				self.block_structure_pending_changed = false;
			}
		}
		Ok(())
	}

	fn storage(&self) -> &S {
		&self.storage
	}

	async fn pins(&mut self) -> Result<&CoMapTransaction<S, String, Pin>, StorageError> {
		self.pins.get().await
	}

	async fn pins_mut(&mut self) -> Result<&mut CoMapTransaction<S, String, Pin>, StorageError> {
		self.pins_changed = true;
		self.pins.get_mut().await
	}

	async fn blocks(&mut self) -> Result<&CoMapTransaction<S, WeakCid, BlockMetadata>, StorageError> {
		self.blocks.get().await
	}

	async fn blocks_mut(&mut self) -> Result<&mut CoMapTransaction<S, WeakCid, BlockMetadata>, StorageError> {
		self.blocks_changed = true;
		self.blocks.get_mut().await
	}

	// async fn blocks_index_unreferenced(&mut self) -> Result<&CoSetTransaction<S, WeakCid>, StorageError> {
	// 	blocks_index_unreferenced.get().await
	// }

	async fn blocks_index_unreferenced_mut(
		&mut self,
	) -> Result<&mut CoMapTransaction<S, WeakCid, BlockInfo>, StorageError> {
		self.blocks_index_unreferenced_changed = true;
		self.blocks_index_unreferenced.get_mut().await
	}

	async fn block_structure_pending_mut(
		&mut self,
	) -> Result<&mut CoMapTransaction<S, WeakCid, BlockStructurePending>, StorageError> {
		self.block_structure_pending_changed = true;
		self.block_structure_pending.get_mut().await
	}
}

async fn reduce<S>(transaction: &mut StorageTransaction<S>, action: StorageAction) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	match action {
		StorageAction::Reference(info, references) => reduce_reference(transaction, info, references).boxed().await?,
		StorageAction::Unreference(info, references) => {
			reduce_unreference(transaction, info, references).boxed().await?
		},
		StorageAction::Structure(structure) => reduce_structure(transaction, structure).boxed().await?,
		StorageAction::ReferenceCreate(info, references) => {
			reduce_reference_create(transaction, info, references).boxed().await?
		},
		StorageAction::Remove(info, cids, zero) => reduce_remove(transaction, cids, zero, info).boxed().await?,
		StorageAction::Delete(info, cids, force) => reduce_delete(transaction, cids, force, info).boxed().await?,
		StorageAction::TagsInsert(cids, tags) => reduce_tags_insert(transaction, cids, tags).boxed().await?,
		StorageAction::TagsRemove(cids, tags) => reduce_tags_remove(transaction, cids, tags).boxed().await?,
		StorageAction::PinCreate(key, strategy, references) => {
			reduce_pin_create(transaction, key, strategy, references).boxed().await?
		},
		StorageAction::PinUpdate(key, strategy) => reduce_pin_update(transaction, key, strategy).boxed().await?,
		StorageAction::PinReference(key, references) => {
			reduce_pin_reference(transaction, key, references).boxed().await?
		},
		StorageAction::PinRemove(key) => reduce_pin_remove(transaction, key).boxed().await?,
		StorageAction::Batch(actions) => {
			let actions_stream = actions.stream(transaction.storage());
			pin_mut!(actions_stream);
			while let Some((_, action)) = actions_stream.try_next().await? {
				Box::pin(reduce(transaction, action)).await?;
			}
		},
	}
	Ok(())
}

/// See: [`StorageAction::ReferenceStructure`]
async fn reduce_structure<S>(
	transaction: &mut StorageTransaction<S>,
	structure: BTreeMap<WeakCid, References>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for (parent, children) in structure {
		reference_structure_cid(transaction, parent, children).await?;
	}
	Ok(())
}

/// Reference/Unreference a children of a recursive reference.
/// When this gets called for Unreference the `parent` block already has been deleted.
async fn reference_structure_cid<S>(
	transaction: &mut StorageTransaction<S>,
	parent: WeakCid,
	children: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// remove pending flag and ignore if not pending
	let pending = match transaction.block_structure_pending_mut().await?.remove(parent).await? {
		Some(info) => info,
		None => {
			return Ok(());
		},
	};
	match pending {
		BlockStructurePending::Reference(info) => {
			// get block
			let mut block = transaction
				.blocks()
				.await?
				.get(&parent)
				.await?
				.ok_or(anyhow!("Reference not found: {:?}", parent))?;

			// reference children
			let info = info.clone().with_block_type(BlockType::Unknown);
			for (child_cid, child_tags) in children.0.into_iter() {
				reference_cid(transaction, &info, child_cid, child_tags.references, &child_tags.tags).await?;
			}

			// mode
			block.mode = ReferenceMode::Recursive;

			// remove
			if block.is_removable() {
				transaction
					.blocks_index_unreferenced_mut()
					.await?
					.insert(parent, info.clone())
					.await?;
			}

			// store
			transaction.blocks_mut().await?.insert(parent, block).await?;
		},
	}
	Ok(())
}

async fn reduce_pin_remove<S>(transaction: &mut StorageTransaction<S>, key: String) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// pin
	let pin = transaction
		.pins_mut()
		.await?
		.remove(key.clone())
		.await?
		.ok_or(anyhow!("Pin not found: {}", key))?;
	let info = BlockInfo::new(transaction.storage(), key.clone(), BlockType::Root).await?;

	// references
	let cids = pin.references.stream(transaction.storage()).map_ok(|(_key, value)| value);
	pin_mut!(cids);
	while let Some(reference) = cids.try_next().await? {
		unreference_cid(transaction, &info, reference, Unreference::ByOne).await?;
	}

	// result
	Ok(())
}

async fn reduce_pin_reference<S>(
	transaction: &mut StorageTransaction<S>,
	key: String,
	references: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// apply
	pin_reference(transaction, key, references).await?;

	// result
	Ok(())
}

async fn pin_reference<S>(
	transaction: &mut StorageTransaction<S>,
	key: String,
	references: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pin = transaction
		.pins()
		.await?
		.get(&key)
		.await?
		.ok_or(anyhow!("Pin not found: {}", key))?;
	let mut pin_references = pin.references.open(transaction.storage()).await?;
	let info = BlockInfo::new(transaction.storage(), key.clone(), BlockType::Root).await?;

	// insert references
	for (reference, block_metadata) in references.0 {
		pin_references.push(reference).await?;
		pin.references_count += 1;
		reference_cid(transaction, &info, reference, block_metadata.references, &block_metadata.tags).await?;
	}

	// apply pin strategy
	apply_pin_strategy(transaction, &mut pin, &mut pin_references, info.clone()).await?;

	// store pin
	pin.references = pin_references.store().await?;
	transaction.pins_mut().await?.insert(key, pin).await?;

	Ok(())
}

/// Apply pin strategy on pin.
async fn apply_pin_strategy<S>(
	transaction: &mut StorageTransaction<S>,
	pin: &mut Pin,
	references: &mut CoListTransaction<S, WeakCid>,
	info: BlockInfo,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut changed = false;
	match &pin.strategy {
		PinStrategy::Unlimited => {},
		PinStrategy::MaxCount(count) => {
			while pin.references_count > *count {
				if let Some((_, remove)) = references.pop_front().await? {
					unreference_cid(transaction, &info, remove, Unreference::ByOne).await?;
				}
				pin.references_count -= 1;
				changed = true;
			}
		},
	}
	Ok(changed)
}

async fn reduce_pin_create<S>(
	transaction: &mut StorageTransaction<S>,
	key: String,
	strategy: PinStrategy,
	references: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// validate
	if transaction.pins().await?.contains_key(&key).await? {
		return Err(anyhow::anyhow!("Pin already exists: {}", key));
	}

	// insert pin
	let pin = Pin { strategy, references: Default::default(), references_count: 0 };
	transaction.pins_mut().await?.insert(key.clone(), pin).await?;

	// initial
	if !references.0.is_empty() {
		pin_reference(transaction, key, references).await?;
	}

	// result
	Ok(())
}

async fn reduce_pin_update<S>(
	transaction: &mut StorageTransaction<S>,
	key: String,
	strategy: PinStrategy,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// get
	let Some(mut pin) = transaction.pins().await?.get(&key).await? else {
		return Err(anyhow::anyhow!("Pin not exists: {}", key));
	};
	let info = BlockInfo::new(transaction.storage(), key.clone(), BlockType::Root).await?;

	// update pin strategy
	pin.strategy = strategy;

	// enfore pin strategy
	let mut references = pin.references.open(transaction.storage()).await?;
	apply_pin_strategy(transaction, &mut pin, &mut references, info).await?;

	// store pin
	pin.references = references.store().await?;
	transaction.pins_mut().await?.insert(key, pin).await?;

	// result
	Ok(())
}

async fn reduce_tags_remove<S>(
	transaction: &mut StorageTransaction<S>,
	cids: BTreeSet<WeakCid>,
	tags: Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for cid in cids {
		transaction
			.blocks_mut()
			.await?
			.try_update_or_insert_async(cid, |mut block| async {
				block.tags.clear(Some(&tags));
				Ok(block)
			})
			.await?;
	}
	Ok(())
}

async fn reduce_tags_insert<S>(
	transaction: &mut StorageTransaction<S>,
	cids: BTreeSet<WeakCid>,
	tags: Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for cid in cids {
		transaction
			.blocks_mut()
			.await?
			.try_update_or_insert_async(cid, |mut block| {
				let mut tags = tags.clone();
				async move {
					block.tags.append(&mut tags);
					Ok(block)
				}
			})
			.await?;
	}
	Ok(())
}

async fn reduce_remove<S>(
	transaction: &mut StorageTransaction<S>,
	cids: impl IntoIterator<Item = WeakCid>,
	zero: bool,
	info: BlockInfo,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for cid in cids {
		unreference_cid(transaction, &info, cid, if zero { Unreference::ToZero } else { Unreference::ByOne }).await?;
	}
	Ok(())
}

/// Delete block references from storage state.
/// After this call the parent blocks can be deleted.
async fn reduce_delete<S>(
	transaction: &mut StorageTransaction<S>,
	cids: impl IntoIterator<Item = (WeakCid, BTreeSet<WeakCid>)>,
	force: bool,
	info: BlockInfo,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// remove
	let info = info.clone().with_block_type(BlockType::Unknown);
	for (cid, links) in cids {
		// structure
		let mut references = References::default();
		references.extend(links.iter().cloned());
		reference_structure_cid(transaction, cid, references).await?;

		// remove block
		let block = match transaction.blocks().await?.get(&cid).await? {
			Some(block) if (block.references == 0 || force) => transaction.blocks_mut().await?.remove(cid).await?,
			_ => None,
		};
		if let Some(block) = block {
			// remove from index
			transaction.blocks_index_unreferenced_mut().await?.remove(cid).await?;

			// unreference links
			match block.mode {
				ReferenceMode::Shallow => {},
				ReferenceMode::Recursive => {
					for link in links.iter() {
						unreference_cid(transaction, &info, *link, Unreference::ByOne).await?;
					}
				},
			}
		}
	}

	// result
	Ok(())
}

async fn reduce_reference<S>(
	transaction: &mut StorageTransaction<S>,
	info: BlockInfo,
	references: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for (reference, data) in references.0.into_iter() {
		reference_cid(transaction, &info, reference, data.references, &data.tags).await?;
	}
	Ok(())
}

async fn reduce_reference_create<S>(
	transaction: &mut StorageTransaction<S>,
	info: BlockInfo,
	references: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for (reference, block_metadata) in references.0 {
		if transaction.blocks().await?.get(&reference).await?.is_none() {
			// we only want to reuse tags here
			// other data is managed internally by the core
			let block = BlockMetadata { tags: block_metadata.tags, ..Default::default() };

			// block
			transaction.blocks_mut().await?.insert(reference, block).await?;

			// shallow
			transaction
				.block_structure_pending_mut()
				.await?
				.insert(reference, BlockStructurePending::Reference(info.clone()))
				.await?;
		}
	}
	Ok(())
}

async fn reference_cid<S>(
	transaction: &mut StorageTransaction<S>,
	info: &BlockInfo,
	cid: WeakCid,
	references: u32,
	tags: &Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let block = transaction.blocks().await?.get(&cid).await?;

	// new block?
	if let Some(block) = &block {
		// remove from index as we have references now
		if block.references == 0 {
			transaction.blocks_index_unreferenced_mut().await?.remove(cid).await?;
		}
	} else {
		// add to pending as we are about to create the block
		transaction
			.block_structure_pending_mut()
			.await?
			.insert(cid, BlockStructurePending::Reference(info.clone()))
			.await?;
	}

	// increment
	let mut block = block.unwrap_or_default();
	block.references += references;
	block.tags.extend(tags.iter().cloned());
	transaction.blocks_mut().await?.insert(cid, block).await?;

	// result
	Ok(())
}

async fn reduce_unreference<S>(
	transaction: &mut StorageTransaction<S>,
	info: BlockInfo,
	references: References,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	for (reference, data) in references.0.into_iter() {
		unreference_cid(transaction, &info, reference, Unreference::By(data.references)).await?;
	}
	Ok(())
}

enum Unreference {
	ByOne,
	By(u32),
	ToZero,
	// To(u32),
}

async fn unreference_cid<S>(
	transaction: &mut StorageTransaction<S>,
	info: &BlockInfo,
	cid: WeakCid,
	reference: Unreference,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(match transaction.blocks().await?.get(&cid).await? {
		Some(mut block) if block.references > 0 => {
			// decrement
			match reference {
				Unreference::ByOne => {
					block.references -= 1;
				},
				Unreference::By(by) => {
					block.references -= by;
				},
				Unreference::ToZero => {
					block.references = 0;
				},
				// Unreference::To(to) => {
				// 	block.references = to;
				// },
			}

			// index
			if block.is_removable() {
				transaction
					.blocks_index_unreferenced_mut()
					.await?
					.insert(cid, info.clone())
					.await?;
			}

			// store
			transaction.blocks_mut().await?.insert(cid, block).await?;

			// result
			true
		},
		_ => false,
	})
}

#[cfg(test)]
mod tests {
	use crate::{PinStrategy, References, Storage, StorageAction};
	use cid::Cid;
	use co_api::{BlockSerializer, BlockStorageExt, CoreBlockStorage, OptionLink, Reducer, ReducerAction, WeakCid};
	use co_storage::MemoryBlockStorage;
	use futures::TryStreamExt;
	use ipld_core::{ipld::Ipld, serde::to_ipld};
	use std::{collections::BTreeMap, str::FromStr};

	#[test]
	fn test_serialize_storage_action() {
		let cid1 = *BlockSerializer::default().serialize(&1).unwrap().cid();
		let cid2 = *BlockSerializer::default().serialize(&2).unwrap().cid();
		let cid3 = *BlockSerializer::default().serialize(&2).unwrap().cid();
		let mut map = BTreeMap::<WeakCid, References>::new();
		map.entry(cid1.into()).or_default().insert(cid2);
		map.entry(cid1.into()).or_default().insert(cid3);

		// action
		let action = StorageAction::Structure(map.into_iter().collect());
		let block = BlockSerializer::default().serialize(&action).unwrap();
		let action_deserialize: StorageAction = BlockSerializer::default().deserialize(&block).unwrap();
		assert_eq!(action_deserialize, action);

		// reducer action
		let reducer_action: ReducerAction<StorageAction> =
			ReducerAction { core: "storage".to_owned(), from: "test".to_owned(), payload: action.clone(), time: 123 };
		let block = BlockSerializer::default().serialize(&reducer_action).unwrap();
		let reducer_action_deserialize: ReducerAction<StorageAction> =
			BlockSerializer::default().deserialize(&block).unwrap();
		assert_eq!(reducer_action_deserialize, reducer_action);

		// reducer action ipld
		let reducer_action_ipld: ReducerAction<Ipld> = ReducerAction {
			core: "storage".to_owned(),
			from: "test".to_owned(),
			payload: to_ipld(action).unwrap(),
			time: 123,
		};
		let reducer_action_ipld_deserialize: ReducerAction<Ipld> =
			BlockSerializer::default().deserialize(&block).unwrap();

		assert_eq!(reducer_action_ipld_deserialize, reducer_action_ipld);
	}

	/// This is data gatered from storage_cleanup test which failed.
	#[tokio::test]
	async fn test_blocks_index_unreferenced_is_correct() {
		fn cid(s: &str) -> co_api::WeakCid {
			Cid::from_str(s).unwrap().into()
		}
		let storage = CoreBlockStorage::new(MemoryBlockStorage::default(), true);

		// actions
		let actions = [
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::PinCreate("co.local.state".into(), PinStrategy::MaxCount(100), [].into()),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::PinCreate("co.local.log".into(), PinStrategy::MaxCount(100), [].into()),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::PinReference(
					"co.local.state".into(),
					[(cid("bagakbqabdyqar5vlsfqd3g4mxngt3yl7nx2na2kb4jybylzn5bktwnihjhih42a"))].into(),
				),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::Structure(
					[
						(
							(cid("bagakbqabdyqar5vlsfqd3g4mxngt3yl7nx2na2kb4jybylzn5bktwnihjhih42a")),
							[
								(cid("QmUDCqxH2vm9MBb2mLsGmHsoCMXBBnd4iWDruZdcSGaN7d")),
								(cid("QmY8fStJQWVsfY4ae7KzfgeJQKqcXEbp1THut3Uz4aBBP6")),
								(cid("QmcS1eGNuBM3a4pf8hw4hEWwdALXEEnimqZBhSBo8aHS7K")),
								(cid("bagakbqabdyqcgkbe7hbegknbemf73xlnooct2g35zzrbdkus6z342bir46k5zgq")),
							]
							.into(),
						),
						(
							(cid("bagakbqabdyqcgkbe7hbegknbemf73xlnooct2g35zzrbdkus6z342bir46k5zgq")),
							[(cid("bagakbqabdyqfomt5rhne4gqclpbi7t2emjthzcm4frymppcndo27rxum6tugwoi"))].into(),
						),
						(
							(cid("bagakbqabdyqfomt5rhne4gqclpbi7t2emjthzcm4frymppcndo27rxum6tugwoi")),
							[(cid("bagakbqabdyqdyybl3osmbp4ckybdvmwccje5kxa6bhy6yz7p3ftrsngh4r6lg5a"))].into(),
						),
					]
					.into(),
				),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::PinReference(
					"co.local.state".into(),
					[(cid("bagakbqabdyqldyp7kxv6p5wb3edrywc74xfkgauqzlumlxncdlzncbwt36y7iby"))].into(),
				),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 1745513086640,
				core: "storage".into(),
				payload: StorageAction::PinUpdate("co.local.state".into(), PinStrategy::MaxCount(1)),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::PinReference(
					"co.local.state".into(),
					[(cid("bagakbqabdyqklkdo5hv4smstsuv2t347nnonrdgylyrb3qepc2rh5p2qtntmbba"))].into(),
				),
			},
			ReducerAction {
				from: "did:local:device".into(),
				time: 0,
				core: "storage".into(),
				payload: StorageAction::Structure(
					[
						(
							(cid("bagakbqabdyqklkdo5hv4smstsuv2t347nnonrdgylyrb3qepc2rh5p2qtntmbba")),
							[(cid("bagakbqabdyqh7c4dgnjexzftz5aethy36hwi4q6iosiwy32e6lortxcp3l6et3a"))].into(),
						),
						(
							(cid("bagakbqabdyqh7c4dgnjexzftz5aethy36hwi4q6iosiwy32e6lortxcp3l6et3a")),
							[
								(cid("bagakbqabdyqc63i6iuxec7qgmzor4a554ihpznnbmnonh2l5l2h6w4vcvyn2zia")),
								(cid("bagakbqabdyqodqxbpakp23ngiqce4hhif2w5n54ujalwomt5lravwfezkdgyica")),
								(cid("bagakbqabdyqosx7w5aag3uid3tgh6w3g7p5vmtykf4cqefg7zwbpkf27bfvqlby")),
							]
							.into(),
						),
						(
							(cid("bagakbqabdyqc63i6iuxec7qgmzor4a554ihpznnbmnonh2l5l2h6w4vcvyn2zia")),
							[(cid("bagakbqabdyqpr2imdfe2lch4cqf7e4cjd5i26yrjsqai2gbwipxdesgukfupu7q"))].into(),
						),
						(
							(cid("bagakbqabdyqodqxbpakp23ngiqce4hhif2w5n54ujalwomt5lravwfezkdgyica")),
							[(cid("bagakbqabdyqjf3zpgq5jg7fjnnxo3pybvf63f7n73now5pvnednflv4ezgahadq"))].into(),
						),
						(
							(cid("bagakbqabdyqosx7w5aag3uid3tgh6w3g7p5vmtykf4cqefg7zwbpkf27bfvqlby")),
							[(cid("bagakbqabdyqebeu7wndmyhr63zfriwlaoddqy3sygd5it7xagora7xreqbbjk3q"))].into(),
						),
						(
							(cid("bagakbqabdyqjf3zpgq5jg7fjnnxo3pybvf63f7n73now5pvnednflv4ezgahadq")),
							[(cid("bagakbqabdyqocquirj4gdy2vvismgm52awzdgf66sqevvrswwvyalg57pt5bboy"))].into(),
						),
						(
							(cid("bagakbqabdyqocquirj4gdy2vvismgm52awzdgf66sqevvrswwvyalg57pt5bboy")),
							[(cid("bagakbqabdyqjamecznbm6ninfi5dryyvshenwnzbiunh7v6qrqy2ydlfkobjakq"))].into(),
						),
					]
					.into(),
				),
			},
		];
		let mut state_reference = OptionLink::none();
		for action in actions {
			let action_link = storage.set_value(&action).await.unwrap();
			state_reference = Storage::reduce(state_reference, action_link, &storage).await.unwrap().into();
		}

		// validate
		let state = storage.get_value(&state_reference.unwrap()).await.unwrap();
		assert!(state
			.blocks_index_unreferenced
			.contains(&storage, &cid("bagakbqabdyqar5vlsfqd3g4mxngt3yl7nx2na2kb4jybylzn5bktwnihjhih42a"))
			.await
			.unwrap());
		assert!(!state
			.blocks_index_unreferenced
			.contains(&storage, &cid("bagakbqabdyqldyp7kxv6p5wb3edrywc74xfkgauqzlumlxncdlzncbwt36y7iby"))
			.await
			.unwrap());
	}

	/// This is data gatered from storage_cleanup test which failed.
	#[tokio::test]
	async fn test_pin_strategy_max() {
		fn cid(s: &str) -> co_api::WeakCid {
			Cid::from_str(s).unwrap().into()
		}
		fn action(s: StorageAction) -> ReducerAction<StorageAction> {
			ReducerAction { from: "did:local:device".into(), time: 0, core: "storage".into(), payload: s }
		}
		let storage = CoreBlockStorage::new(MemoryBlockStorage::default(), true);

		// actions
		let actions = [
			action(StorageAction::PinCreate("co:local".into(), PinStrategy::MaxCount(100), [].into())),
			action(StorageAction::PinReference(
				"co:local".into(),
				[(cid("bafyr4idmz6tdkhmdwhis4w2yov4g7ctjs72bcixzk2q7m3ioihhm4lvnky"))].into(),
			)),
			action(StorageAction::PinReference(
				"co:local".into(),
				[
					(cid("bafyr4id6ivgo6penzkew6tv2jsnncuq7a3zm7ajqd4nfmuxry7tq6xawbq")),
					(cid("bafyr4ih47p3rp5ppftduphy2fikph63iey5fwv42du6eyjccyu3ygvqvzy")),
				]
				.into(),
			)),
			action(StorageAction::PinUpdate("co:local".into(), PinStrategy::MaxCount(1))),
			action(StorageAction::PinReference(
				"co:local".into(),
				[
					(cid("bafyr4igconcuuuokydue7wglesze5vdyahzprgpkn7ukajd76besyhw2mi")),
					(cid("bafyr4iegqvwuhpdfp6vdfyxpxbm4qfjjo5y4rko34j6s7eqf2xfijo5chy")),
				]
				.into(),
			)),
		];
		let mut state_reference = OptionLink::none();
		for action in actions {
			let action_link = storage.set_value(&action).await.unwrap();
			state_reference = Storage::reduce(state_reference, action_link, &storage).await.unwrap().into();
		}

		// validate
		let state = storage.get_value(&state_reference.unwrap()).await.unwrap();
		let pin = state.pins.get(&storage, &"co:local".to_owned()).await.unwrap().unwrap();
		let pin = pin
			.references
			.stream(&storage)
			.map_ok(|(_index, value)| value)
			.try_collect::<Vec<_>>()
			.await
			.unwrap();
		assert_eq!(pin, vec![cid("bafyr4iegqvwuhpdfp6vdfyxpxbm4qfjjo5y4rko34j6s7eqf2xfijo5chy")]);
	}
}
