use anyhow::anyhow;
use cid::Cid;
use co_api::{
	async_api::Reducer, BlockStorage, BlockStorageExt, CoList, CoMap, CoMapTransaction, Link, OptionLink,
	ReducerAction, StorageError, Tags,
};
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Storage {
	/// Named pins.
	#[serde(rename = "p")]
	pub pins: CoMap<String, Pin>,

	/// Block metadata.
	#[serde(rename = "b")]
	pub blocks: CoMap<Cid, BlockMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockMetadata {
	#[serde(rename = "r")]
	pub references: u32,

	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pin {
	/// Free strategy.
	#[serde(rename = "s")]
	strategy: PinStrategy,

	/// Pinned references.
	/// Sorted by insertion (oldest is first).
	/// Every pinned item will automatically maintain a reference count.
	#[serde(rename = "r")]
	references: CoList<Cid>,

	/// Pinned references count.
	#[serde(rename = "c")]
	references_count: u32,
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
	/// Non recursive: [`Cid`] links are not added automatically.
	#[serde(rename = "r")]
	Reference(Vec<Cid>),

	/// Decrease [`Cid`] reference count by one.
	#[serde(rename = "u")]
	Unreference(Vec<Cid>),

	/// Remove [`Cid`] references.
	///
	/// # Arguments
	/// - `0`: The [`Cid`] of entries to remove.
	/// - `1`: Force removal. If false only references with a zero ref count will be removed.
	#[serde(rename = "d")]
	Remove(Vec<Cid>, bool),

	/// Append tags to references.
	#[serde(rename = "ti")]
	TagsInsert(Vec<Cid>, Tags),

	/// Remove tags from references.
	#[serde(rename = "tr")]
	TagsRemove(Vec<Cid>, Tags),

	/// Create a named pin and reference all specified [`Cid`]s.
	PinCreate(String, Pin),

	/// Insert references to a named pin and reference all specified [`Cid`]s.
	PinReference(String, Vec<Cid>),

	/// Remove a named pin and unreference all [`Cid`]s.
	PinRemove(String),
}

impl<S: BlockStorage + Clone + 'static> Reducer<StorageAction, S> for Storage {
	async fn reduce(
		state: OptionLink<Self>,
		event: ReducerAction<StorageAction>,
		storage: &S,
	) -> Result<Link<Self>, anyhow::Error> {
		let mut state = storage.get_value_or_default(&state).await?;
		match event.payload {
			StorageAction::Reference(cids) => reference(storage, &mut state, stream::iter(cids).map(Ok)).await?,
			StorageAction::Unreference(cids) => unreference(storage, &mut state, stream::iter(cids).map(Ok)).await?,
			StorageAction::Remove(cids, force) => reduce_remove(storage, &mut state, cids, force).await?,
			StorageAction::TagsInsert(cids, tags) => reduce_tags_insert(storage, &mut state, cids, tags).await?,
			StorageAction::TagsRemove(cids, tags) => reduce_tags_remove(storage, &mut state, cids, tags).await?,
			StorageAction::PinCreate(key, pin) => reduce_pin_create(storage, &mut state, key, pin).await?,
			StorageAction::PinReference(key, cids) => reduce_pin_reference(storage, &mut state, key, cids).await?,
			StorageAction::PinRemove(key) => reduce_pin_remove(storage, &mut state, key).await?,
		}
		Ok(storage.set_value(&state).await?)
	}
}

async fn reduce_pin_remove<S>(storage: &S, state: &mut Storage, key: String) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;

	// pin
	let pin = pins.remove(key.clone()).await?.ok_or(anyhow!("Pin not found: {}", key))?;

	// references
	unreference(storage, state, pin.references.stream(storage).map_ok(|(_key, value)| value)).await?;

	// store
	state.pins = pins.store().await?;

	// result
	Ok(())
}

async fn reduce_pin_reference<S>(
	storage: &S,
	state: &mut Storage,
	key: String,
	cids: Vec<Cid>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;
	let mut blocks = state.blocks.open(storage).await?;

	// apply
	let mut pin = pins.get(&key).await?.ok_or(anyhow!("Pin not found: {}", key))?;
	let mut references = pin.references.open(storage).await?;
	for cid in &cids {
		references.push(*cid).await?;
		pin.references_count += 1;
		reference_cid(&mut blocks, *cid).await?;
	}
	match &pin.strategy {
		PinStrategy::Unlimited => {},
		PinStrategy::MaxCount(count) => {
			while pin.references_count > *count {
				if let Some((_, remove)) = references.pop_front().await? {
					unreference_cid(&mut blocks, remove).await?;
				}
				pin.references_count -= 1;
			}
		},
	}
	pin.references = references.store().await?;
	pins.insert(key, pin).await?;

	// store
	state.pins = pins.store().await?;
	state.blocks = blocks.store().await?;

	// result
	Ok(())
}

async fn reduce_pin_create<S>(storage: &S, state: &mut Storage, key: String, mut pin: Pin) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut pins = state.pins.open(storage).await?;
	let mut blocks = state.blocks.open(storage).await?;

	// validate
	if !pins.contains_key(&key).await? {
		return Err(anyhow::anyhow!("Pin already exists: {}", key));
	}

	// references
	pin.references_count = 0;
	{
		let references = pin.references.stream(storage);
		pin_mut!(references);
		while let Some((_, cid)) = references.try_next().await? {
			reference_cid(&mut blocks, cid).await?;
			pin.references_count += 1;
		}
	}

	// insert pin
	pins.insert(key, pin).await?;

	// store
	state.pins = pins.store().await?;
	state.blocks = blocks.store().await?;

	// result
	Ok(())
}

async fn reduce_tags_remove<S>(
	storage: &S,
	state: &mut Storage,
	cids: Vec<Cid>,
	tags: Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	for cid in cids {
		blocks
			.update_key(cid, |mut block| async {
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
	cids: Vec<Cid>,
	tags: Tags,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	for cid in cids {
		blocks
			.update_key(cid, |mut block| {
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
	cids: Vec<cid::CidGeneric<64>>,
	force: bool,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	for cid in cids {
		if force || blocks.get(&cid).await?.unwrap_or_default().references == 0 {
			blocks.remove(cid).await?;
		}
	}
	state.blocks = blocks.store().await?;
	Ok(())
}

async fn reference<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl Stream<Item = Result<Cid, StorageError>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	pin_mut!(cids);
	while let Some(cid) = cids.try_next().await? {
		reference_cid(&mut blocks, cid).await?;
	}
	state.blocks = blocks.store().await?;
	Ok(())
}

async fn reference_cid<S>(blocks: &mut CoMapTransaction<S, Cid, BlockMetadata>, cid: Cid) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	blocks
		.update_key(cid, |mut block| async move {
			block.references += 1;
			Ok(block)
		})
		.await?;
	Ok(())
}

async fn unreference<S>(
	storage: &S,
	state: &mut Storage,
	cids: impl Stream<Item = Result<Cid, StorageError>>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	let mut blocks = state.blocks.open(storage).await?;
	pin_mut!(cids);
	while let Some(cid) = cids.try_next().await? {
		unreference_cid(&mut blocks, cid).await?;
	}
	state.blocks = blocks.store().await?;
	Ok(())
}

async fn unreference_cid<S>(
	blocks: &mut CoMapTransaction<S, Cid, BlockMetadata>,
	cid: Cid,
) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(if let Some(mut block) = blocks.get(&cid).await? {
		if block.references > 0 {
			block.references -= 1;
			blocks.insert(cid, block).await?;
			true
		} else {
			false
		}
	} else {
		false
	})
}
