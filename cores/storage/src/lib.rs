use anyhow::anyhow;
use co_api::{
	async_api::Reducer, BlockStorage, BlockStorageExt, CoList, CoListTransaction, CoMap, CoMapTransaction, CoSet,
	CoSetTransaction, Link, OptionLink, ReducerAction, StorageError, Tags, WeakCid,
};
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Storage {
	/// Named pins.
	#[serde(rename = "p", default, skip_serializing_if = "CoMap::is_empty")]
	pub pins: CoMap<String, Pin>,

	/// Block metadata.
	#[serde(rename = "b", default, skip_serializing_if = "CoMap::is_empty")]
	pub blocks: CoMap<WeakCid, BlockMetadata>,

	/// Block metadata index to unreferenced (reference count of zero) entries.
	#[serde(rename = "bu", default, skip_serializing_if = "CoSet::is_empty")]
	pub blocks_index_unreferenced: CoSet<WeakCid>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockMetadata {
	/// Current reference count on this node.
	#[serde(rename = "r")]
	pub references: u32,

	/// Structural references. Children of this reference.
	/// Every children listed here increases its respective reference count by one.
	#[serde(rename = "c", default, skip_serializing_if = "CoSet::is_empty")]
	pub children: CoSet<WeakCid>,

	/// Additional metadata.
	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PinStrategy {
	/// Unlimited pins.
	#[default]
	#[serde(rename = "u")]
	Unlimited,

	/// Maximum count of references.
	#[serde(rename = "h")]
	MaxCount(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageAction {
	/// Increase [`Cid`] reference count by one.
	/// A single [`Cid`] is allowed to be contained multiple times (=reference count).
	/// The Shallow: [`Cid`] links are not added automatically (not recusrive).
	#[serde(rename = "r")]
	Reference(Vec<WeakCid>),

	/// Decrease [`Cid`] reference count by one.
	#[serde(rename = "u")]
	Unreference(Vec<WeakCid>),

	/// Structurally reference [`Cid`].
	/// The first item is the parent the second is the children to be structurally referenced.
	/// All unique children have their reference count increased by one (idempotent).
	#[serde(rename = "s")]
	ReferenceStructure(Vec<(WeakCid, BTreeSet<WeakCid>)>),

	/// Create [`Cid`] references with ref count of zero if the reference not exists yet.
	/// This is normally used to track newly created blocks.
	///
	/// # Arguments
	/// - `0`: The [`Cid`] of entries to create.
	#[serde(rename = "c")]
	ReferenceCreate(BTreeSet<WeakCid>),

	/// Remove [`Cid`] references.
	///
	/// # Arguments
	/// - `0`: The [`Cid`] of entries to remove.
	/// - `1`: Force removal. If false only references with a zero ref count will be removed.
	#[serde(rename = "d")]
	Remove(BTreeSet<WeakCid>, bool),

	/// Append tags to references.
	#[serde(rename = "ti")]
	TagsInsert(Vec<WeakCid>, Tags),

	/// Remove tags from references.
	#[serde(rename = "tr")]
	TagsRemove(Vec<WeakCid>, Tags),

	/// Create a named pin and reference all specified [`Cid`]s.
	#[serde(rename = "pc")]
	PinCreate(String, PinStrategy, Vec<WeakCid>),

	/// Update a named pin by setting the [`PinStrategy`].
	#[serde(rename = "pu")]
	PinUpdate(String, PinStrategy),

	/// Insert references to a named pin and reference all specified [`Cid`]s.
	#[serde(rename = "pr")]
	PinReference(String, Vec<WeakCid>),

	/// Remove a named pin and unreference all [`Cid`]s.
	#[serde(rename = "pd")]
	PinRemove(String),
}
impl Storage {
	/// Create inital state.
	pub async fn initial_state<S: BlockStorage + Clone + 'static>(
		storage: &S,
		actions: Vec<StorageAction>,
	) -> Result<OptionLink<Self>, anyhow::Error> {
		let mut state = OptionLink::none();
		for action in actions {
			state = Self::reduce(
				state,
				ReducerAction { from: "".to_owned(), time: 0, core: "".to_owned(), payload: action },
				storage,
			)
			.await?
			.into();
		}
		Ok(state)
	}
}
impl<S: BlockStorage + Clone + 'static> Reducer<StorageAction, S> for Storage {
	async fn reduce(
		state: OptionLink<Self>,
		event: ReducerAction<StorageAction>,
		storage: &S,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut state = storage.get_value_or_default(&state).await?;
		match event.payload {
			StorageAction::Reference(cids) => reduce_reference(storage, &mut state, stream::iter(cids).map(Ok)).await?,
			StorageAction::Unreference(cids) => {
				reduce_unreference(storage, &mut state, stream::iter(cids).map(Ok)).await?
			},
			StorageAction::ReferenceStructure(cids) => {
				reduce_reference_structure(storage, &mut state, stream::iter(cids).map(Ok)).await?
			},
			StorageAction::ReferenceCreate(cids) => {
				reduce_reference_create(storage, &mut state, stream::iter(cids).map(Ok)).await?
			},
			StorageAction::Remove(cids, force) => reduce_remove(storage, &mut state, cids, force).await?,
			StorageAction::TagsInsert(cids, tags) => reduce_tags_insert(storage, &mut state, cids, tags).await?,
			StorageAction::TagsRemove(cids, tags) => reduce_tags_remove(storage, &mut state, cids, tags).await?,
			StorageAction::PinCreate(key, strategy, references) => {
				reduce_pin_create(storage, &mut state, key, strategy, references).await?
			},
			StorageAction::PinUpdate(key, strategy) => reduce_pin_update(storage, &mut state, key, strategy).await?,
			StorageAction::PinReference(key, cids) => reduce_pin_reference(storage, &mut state, key, cids).await?,
			StorageAction::PinRemove(key) => reduce_pin_remove(storage, &mut state, key).await?,
		}
		Ok(storage.set_value(&state).await?)
	}
}

/// See: [`StorageAction::ReferenceStructure`]
async fn reduce_reference_structure<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl Stream<Item = Result<(WeakCid, BTreeSet<WeakCid>), StorageError>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;
	pin_mut!(cids);
	while let Some((parent, children)) = cids.try_next().await? {
		reference_structure_cid(storage, &mut blocks, &mut blocks_index_unreferenced, parent, children).await?;
	}
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;
	Ok(())
}

/// Add unique children to parent block metadata.
async fn reference_structure_cid<S>(
	storage: &S,
	blocks: &mut CoMapTransaction<S, WeakCid, BlockMetadata>,
	blocks_index_unreferenced: &mut CoSetTransaction<S, WeakCid>,
	parent: WeakCid,
	children: BTreeSet<WeakCid>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut block = blocks.get(&parent).await?.ok_or(anyhow!("Reference not found: {:?}", parent))?;

	// add children
	if !children.is_empty() {
		let mut children_transaction = block.children.open(storage).await?;
		let mut changed = false;
		for item in children {
			let item = item.into();
			if !children_transaction.contains(&item).await? {
				changed = true;

				// add children
				children_transaction.insert(item).await?;

				// reference
				reference_cid(blocks, blocks_index_unreferenced, item).await?;
			}
		}

		// store
		if changed {
			block.children = children_transaction.store().await?;
			blocks.insert(parent, block).await?;
		}
	}

	Ok(())
}

async fn reduce_pin_remove<S>(storage: &S, state: &mut Storage, key: String) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;

	// pin
	let pin = pins.remove(key.clone()).await?.ok_or(anyhow!("Pin not found: {}", key))?;

	// references
	reduce_unreference(storage, state, pin.references.stream(storage).map_ok(|(_key, value)| value.into())).await?;

	// store
	state.pins = pins.store().await?;

	// result
	Ok(())
}

async fn reduce_pin_reference<S>(
	storage: &S,
	state: &mut Storage,
	key: String,
	cids: Vec<WeakCid>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;

	// apply
	pin_reference(storage, &mut pins, &mut blocks, &mut blocks_index_unreferenced, key, cids).await?;

	// store
	state.pins = pins.store().await?;
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;

	// result
	Ok(())
}

async fn pin_reference<S>(
	storage: &S,
	pins: &mut CoMapTransaction<S, String, Pin>,
	blocks: &mut CoMapTransaction<S, WeakCid, BlockMetadata>,
	blocks_index_unreferenced: &mut CoSetTransaction<S, WeakCid>,
	key: String,
	cids: Vec<WeakCid>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pin = pins.get(&key).await?.ok_or(anyhow!("Pin not found: {}", key))?;
	let mut references = pin.references.open(storage).await?;

	// insert references
	for cid in cids {
		let cid = cid.into();
		references.push(cid).await?;
		pin.references_count += 1;
		reference_cid(blocks, blocks_index_unreferenced, cid).await?;
	}

	// apply pin strategy
	apply_pin_strategy(blocks, blocks_index_unreferenced, &mut pin, &mut references).await?;

	// store pin
	pin.references = references.store().await?;
	pins.insert(key, pin).await?;

	Ok(())
}

/// Apply pin strategy on pin.
async fn apply_pin_strategy<S>(
	blocks: &mut CoMapTransaction<S, WeakCid, BlockMetadata>,
	blocks_index_unreferenced: &mut CoSetTransaction<S, WeakCid>,
	pin: &mut Pin,
	references: &mut CoListTransaction<S, WeakCid>,
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
					unreference_cid(blocks, blocks_index_unreferenced, remove).await?;
				}
				pin.references_count -= 1;
				changed = true;
			}
		},
	}
	Ok(changed)
}

async fn reduce_pin_create<S>(
	storage: &S,
	state: &mut Storage,
	key: String,
	strategy: PinStrategy,
	references: Vec<WeakCid>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;

	// validate
	if pins.contains_key(&key).await? {
		return Err(anyhow::anyhow!("Pin already exists: {}", key));
	}

	// insert pin
	let pin = Pin { strategy, references: Default::default(), references_count: 0 };
	pins.insert(key.clone(), pin).await?;

	// initial
	if !references.is_empty() {
		pin_reference(storage, &mut pins, &mut blocks, &mut blocks_index_unreferenced, key, references).await?;
	}

	// store
	state.pins = pins.store().await?;
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;

	// result
	Ok(())
}

async fn reduce_pin_update<S>(
	storage: &S,
	state: &mut Storage,
	key: String,
	strategy: PinStrategy,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;

	// get
	let Some(mut pin) = pins.get(&key).await? else {
		return Err(anyhow::anyhow!("Pin not exists: {}", key));
	};

	// update pin strategy
	pin.strategy = strategy;

	// enfore pin strategy
	let mut references = pin.references.open(storage).await?;
	apply_pin_strategy(&mut blocks, &mut blocks_index_unreferenced, &mut pin, &mut references).await?;

	// store pin
	pin.references = references.store().await?;
	pins.insert(key, pin).await?;

	// store
	state.pins = pins.store().await?;
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;

	// result
	Ok(())
}

async fn reduce_tags_remove<S>(
	storage: &S,
	state: &mut Storage,
	cids: Vec<WeakCid>,
	tags: Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	for cid in cids {
		blocks
			.update_key(cid.into(), |mut block| async {
				block.tags.clear(Some(&tags));
				Ok(block)
			})
			.await?;
	}
	state.blocks = blocks.store().await?;
	Ok(())
}

async fn reduce_tags_insert<S>(
	storage: &S,
	state: &mut Storage,
	cids: Vec<WeakCid>,
	tags: Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	for cid in cids {
		blocks
			.update_key(cid.into(), |mut block| {
				let mut tags = tags.clone();
				async move {
					block.tags.append(&mut tags);
					Ok(block)
				}
			})
			.await?;
	}
	state.blocks = blocks.store().await?;
	Ok(())
}

async fn reduce_remove<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl IntoIterator<Item = WeakCid>,
	force: bool,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;

	// remove
	let mut remove_structural = BTreeSet::new();
	for cid in cids {
		let cid = cid.into();

		// remove block
		let block = match blocks.get(&cid).await? {
			Some(block) if block.references == 0 => blocks.remove(cid).await?,
			Some(_) if force => {
				// remove structural references from parents
				//  this is only the case when force remove blocks as it still has references
				remove_structural.insert(cid);

				// remove
				blocks.remove(cid).await?
			},
			_ => None,
		};
		if let Some(block) = block {
			// index
			blocks_index_unreferenced.remove(cid).await?;

			// unreference children
			let children = block.children.stream(storage);
			pin_mut!(children);
			while let Some(cid) = children.try_next().await? {
				unreference_cid(&mut blocks, &mut blocks_index_unreferenced, cid).await?;
			}
		}
	}

	// remove structural references from parents
	if !remove_structural.is_empty() {
		let mut changed_blocks = HashMap::new();

		// scan all blocks for structural referernces to the removed
		// Complexity: `BLOCKS = O(n), C = O(m), O(n * m)`
		{
			let stream = blocks.stream();
			pin_mut!(stream);
			while let Some((cid, mut block)) = stream.try_next().await? {
				let mut children = block.children.open(storage).await?;
				let mut change = false;
				for item in &remove_structural {
					if children.remove(*item).await? {
						change = true;
					}
				}
				if change {
					block.children = children.store().await?;
					changed_blocks.insert(cid, block);
				}
			}
		}

		// replace changed blocks
		for (cid, block) in changed_blocks {
			blocks.insert(cid, block).await?;
		}
	}

	// store
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;

	// result
	Ok(())
}

async fn reduce_reference<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl Stream<Item = Result<WeakCid, StorageError>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;
	pin_mut!(cids);
	while let Some(cid) = cids.try_next().await? {
		reference_cid(&mut blocks, &mut blocks_index_unreferenced, cid.into()).await?;
	}
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;
	Ok(())
}

async fn reduce_reference_create<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl Stream<Item = Result<WeakCid, StorageError>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	pin_mut!(cids);
	while let Some(cid) = cids.try_next().await? {
		let weak_cid = cid.into();
		if blocks.get(&weak_cid).await?.is_none() {
			blocks.insert(weak_cid, BlockMetadata::default()).await?;
		}
	}
	state.blocks = blocks.store().await?;
	Ok(())
}

async fn reference_cid<S>(
	blocks: &mut CoMapTransaction<S, WeakCid, BlockMetadata>,
	blocks_index_unreferenced: &mut CoSetTransaction<S, WeakCid>,
	cid: WeakCid,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let block = blocks.get(&cid).await?;

	// remove from index as we have references now
	if let Some(block) = &block {
		if block.references == 0 {
			blocks_index_unreferenced.remove(cid).await?;
		}
	}

	// increment
	let mut block = block.unwrap_or_default();
	block.references += 1;
	blocks.insert(cid, block).await?;

	// result
	Ok(())
}

async fn reduce_unreference<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl Stream<Item = Result<WeakCid, StorageError>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	let mut blocks_index_unreferenced = state.blocks_index_unreferenced.open(storage).await?;
	pin_mut!(cids);
	while let Some(cid) = cids.try_next().await? {
		unreference_cid(&mut blocks, &mut blocks_index_unreferenced, cid.into()).await?;
	}
	state.blocks = blocks.store().await?;
	state.blocks_index_unreferenced = blocks_index_unreferenced.store().await?;
	Ok(())
}

async fn unreference_cid<S>(
	blocks: &mut CoMapTransaction<S, WeakCid, BlockMetadata>,
	blocks_index_unreferenced: &mut CoSetTransaction<S, WeakCid>,
	cid: WeakCid,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(match blocks.get(&cid).await? {
		Some(mut block) if block.references > 0 => {
			// decrement
			block.references -= 1;

			// index
			if block.references == 0 {
				blocks_index_unreferenced.insert(cid).await?;
			}

			// store
			blocks.insert(cid.clone(), block).await?;

			// result
			true
		},
		_ => false,
	})
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::async_api::reduce::<Storage, StorageAction>()
}

#[cfg(test)]
mod tests {
	use crate::{PinStrategy, Storage, StorageAction};
	use cid::Cid;
	use co_api::{async_api::Reducer, BlockSerializer, BlockStorageExt, OptionLink, ReducerAction, WeakCid};
	use co_storage::MemoryBlockStorage;
	use ipld_core::{ipld::Ipld, serde::to_ipld};
	use std::{
		collections::{BTreeMap, BTreeSet},
		str::FromStr,
	};

	#[test]
	fn test_serialize_storage_action() {
		let cid1 = BlockSerializer::default().serialize(&1).unwrap().cid().clone();
		let cid2 = BlockSerializer::default().serialize(&2).unwrap().cid().clone();
		let cid3 = BlockSerializer::default().serialize(&2).unwrap().cid().clone();
		let mut map = BTreeMap::<WeakCid, BTreeSet<WeakCid>>::new();
		map.entry(cid1.into()).or_default().insert(cid2.into());
		map.entry(cid1.into()).or_default().insert(cid3.into());

		// action
		let action = StorageAction::ReferenceStructure(map.into_iter().collect());
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
		let storage = MemoryBlockStorage::default();

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
				payload: StorageAction::ReferenceStructure(
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
				payload: StorageAction::ReferenceStructure(
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
			state_reference = Storage::reduce(state_reference, action, &storage).await.unwrap().into();
		}

		// validate
		let state = storage.get_value(&state_reference.unwrap()).await.unwrap();
		assert!(state
			.blocks_index_unreferenced
			.contains(&storage, &cid("bagakbqabdyqar5vlsfqd3g4mxngt3yl7nx2na2kb4jybylzn5bktwnihjhih42a"))
			.await
			.unwrap());
		assert!(state
			.blocks_index_unreferenced
			.contains(&storage, &cid("bagakbqabdyqldyp7kxv6p5wb3edrywc74xfkgauqzlumlxncdlzncbwt36y7iby"))
			.await
			.unwrap());
	}
}
