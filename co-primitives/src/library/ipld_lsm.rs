use crate::{BlockStorage, BlockStorageExt, Link, Node, NodeBuilder, NodeStream, OptionLink, StorageError};
use either::Either;
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	borrow::Cow,
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
	future::ready,
	hash::Hash,
	mem::swap,
	num::TryFromIntError,
};

/// LSM Tree Root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Levels.
	#[serde(rename = "l")]
	pub levels: Vec<Link<Level<K, V>>>,

	/// Active "in memory data".
	#[serde(rename = "a")]
	pub active: OptionLink<Node<(K, Value<V>)>>,

	/// Tree settings.
	#[serde(rename = "s")]
	pub settings: TreeSettings,
}

/// LSM Tree Level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	#[serde(rename = "r")]
	pub runs: OptionLink<Node<Run<K, V>>>,
}

/// LSM Tree Run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	#[serde(rename = "e")]
	pub entries: OptionLink<Node<(K, Value<V>)>>,
	#[serde(rename = "i")]
	pub bloom: Vec<u8>,
	#[serde(rename = "l")]
	pub min_key: K,
	#[serde(rename = "h")]
	pub max_key: K,
}
impl<K, V> Run<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Tests if the run possibly contains an key.
	/// TODO: Add bloom filter
	fn may_contains_key(&self, key: &K) -> bool {
		key >= &self.min_key && key <= &self.max_key
	}
}

// /// LSM Tree Sorted Data Block Node.
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum Node<K, V>
// where
// 	K: Hash + Ord + Clone + 'static,
// 	V: Clone + 'static,
// {
// 	#[serde(rename = "n")]
// 	Internal(Vec<Link<Self>>),

// 	#[serde(rename = "l")]
// 	Leaf(BTreeMap<K, V>),
// }
// impl<K, V> Node<K, V>
// where
// 	K: Hash + Ord + Clone + 'static,
// 	V: Clone + 'static,
// {
// 	fn may_contains_key(&self, key: &K) -> bool {
// 		key >= &self.min_key && key <= &self.max_key
// 	}
// }

/// LSM Tree Value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value<V>
where
	V: Clone + 'static,
{
	Value(V),
	Tombstone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSettings {
	/// Max entries (K/V pairs) count in a leaf node.
	#[serde(rename = "n", default = "TreeSettings::default_max_node_entries")]
	pub max_node_entries: u64,

	/// Limits entries (K/V pairs) in active (in memory) run.
	/// Overruning this limit will cause a new L0 run gets created.
	#[serde(rename = "a", default = "TreeSettings::default_max_active_entries")]
	pub max_active_entries: u64,

	/// Limits runs in a level.
	/// Overruning this limit will cause a compaction to next level.
	#[serde(rename = "r", default = "TreeSettings::default_max_run_count")]
	pub max_run_count: u64,
}
impl TreeSettings {
	pub fn default_max_node_entries() -> u64 {
		2 ^ 8 // 256
	}
	pub fn default_max_active_entries() -> u64 {
		2 ^ 14 // 16k
	}
	pub fn default_max_run_count() -> u64 {
		2 ^ 4 // 16
	}
}
impl Default for TreeSettings {
	fn default() -> Self {
		Self {
			max_node_entries: Self::default_max_node_entries(),
			max_active_entries: Self::default_max_active_entries(),
			max_run_count: Self::default_max_run_count(),
		}
	}
}

/// LSM Tree Instance.
pub struct Tree<S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Storage instance.
	storage: S,

	/// LSM Root.
	root: OptionLink<Root<K, V>>,

	/// Active in memory pairs.
	active: BTreeMap<K, Value<V>>,

	// Limits items in memory run.
	settings: TreeSettings,
	// level_cache: Arc<RwLock<BTreeMap<Cid, Level<K, V>>>>,
	// run_cache: Arc<RwLock<BTreeMap<Cid, Run<K, V>>>>,
}
impl<S, K, V> Tree<S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Create new empty tree.
	pub fn new(storage: S, settings: TreeSettings) -> Self {
		Self { active: Default::default(), settings, root: OptionLink::none(), storage }
	}

	/// Load Tree from root CID.
	pub async fn load(storage: S, root: Link<Root<K, V>>) -> Result<Self, StorageError> {
		let mut result = Self { active: Default::default(), settings: Default::default(), root: root.into(), storage };
		if let Some(root) = result.root().await? {
			result.settings = root.settings;
			result.active = NodeStream::from_link(result.storage.clone(), root.active).try_collect().await?;
		}
		Ok(result)
	}

	/// Insert/Replace key.
	pub async fn insert(&mut self, key: K, value: V) -> Result<(), StorageError> {
		// insert entry to memory run
		self.active.insert(key, Value::Value(value));

		// flush?
		if self.active.len() > self.settings.max_active_entries as usize {
			self.flush_active().await?;
		}

		// result
		Ok(())
	}

	/// Remove key.
	pub async fn remove(&mut self, key: K) -> Result<(), StorageError> {
		// insert tombstone to memory run
		self.active.insert(key, Value::Tombstone);

		// flush?
		if self.active.len() > self.settings.max_active_entries as usize {
			self.flush_active().await?;
		}

		// result
		Ok(())
	}

	/// Get value for key.
	pub async fn get(&self, key: &K) -> Result<Option<V>, StorageError> {
		match self.active.get(key) {
			Some(Value::Value(v)) => Ok(Some(v.clone())),
			Some(Value::Tombstone) => Ok(None),
			None => {
				// iterate runs (most up-to-date is the first)
				let runs = self.runs();
				pin_mut!(runs);
				while let Some(run) = runs.try_next().await? {
					if run.may_contains_key(key) {
						// iterate entries (sorted by key)
						// TODO: implement binary search (and/or sparse index)
						let entries = NodeStream::from_link(self.storage.clone(), run.entries);
						pin_mut!(entries);
						while let Some(entry) = entries.try_next().await? {
							if &entry.0 == key {
								return match entry.1 {
									Value::Value(value) => Ok(Some(value)),
									Value::Tombstone => Ok(None),
								};
							}
						}
					}
				}
				Ok(None)
			},
		}
	}

	pub async fn contains_key(&self, key: &K) -> Result<bool, StorageError> {
		Ok(self.get(key).await?.is_some())
	}

	/// Stream all tree entries.
	pub fn stream(&self) -> impl Stream<Item = Result<(K, V), StorageError>> + '_ {
		self.create_stream(None).try_filter_map(|item| {
			ready(Ok(match item.1 {
				Value::Value(value) => Some((item.0, value)),
				Value::Tombstone => None,
			}))
		})
	}

	/// Iterate on runs.
	fn create_stream(
		&self,
		only_run_indicies: Option<BTreeSet<usize>>,
	) -> impl Stream<Item = Result<(K, Value<V>), StorageError>> + '_ {
		let storage = self.storage.clone();
		async_stream::try_stream! {
			// heap (max-sorted)
			let mut heap = match only_run_indicies {
				Some(_) => BinaryHeap::new(),
				None => BinaryHeap::<TreeStreamItem<K, V>>::from(
					self.active
						.clone()
						.into_iter()
						.map(|item| TreeStreamItem { run: None, item })
						.collect::<Vec<_>>(),
				),
			};

			// runs
			//  filter and pop first item of each run
			let mut runs: Vec<(usize, NodeStream<S, _>)> = self
				.runs()
				.enumerate()
				.filter(|(index, _)| ready(match &only_run_indicies {
					Some(run_indicies) => run_indicies.contains(index),
					None => true,
				}))
				.map(|(index, run)| match run {
					Ok(run) => Ok((index, NodeStream::from_link(storage.clone(), run.entries))),
					Err(err) => Err(err),
				})
				.try_collect()
				.await?;
			for (run_index, run) in runs.iter_mut() {
				if let Some(item) = run.try_next().await? {
					heap.push(TreeStreamItem { run: Some(*run_index), item });
				}
			}

			// walk tree
			while let Some(item) = Self::pop_and_fetch(&mut heap, &mut runs).await? {
				// pop superseded keys
				//  we sort the TreeStreamItem by key and run index so we receive the next key front the most recent run first
				while let Some(next_item) = heap.peek() {
					if next_item.item.0 == item.item.0 {
						Self::pop_and_fetch(&mut heap, &mut runs).await?;
					} else {
						break;
					}
				}

				// yield values
				yield item.item;
			}
		}
		// TreeStream::new(self)
	}

	/// Pop item and continue to read the run.
	async fn pop_and_fetch(
		heap: &mut BinaryHeap<TreeStreamItem<K, V>>,
		runs: &mut Vec<(usize, NodeStream<S, (K, Value<V>)>)>,
	) -> Result<Option<TreeStreamItem<K, V>>, StorageError> {
		if let Some(item) = heap.pop() {
			// fetch next
			//  every time we take an item from an run we fetch the next one
			//  so the heap can determine which are the next in sequence
			//  once a run runs out of items it will not be asked again as all run items poped
			if let Some(run_index) = item.run {
				if let Some((_, run)) = runs.get_mut(run_index) {
					if let Some(item) = run.try_next().await? {
						heap.push(TreeStreamItem { run: Some(run_index), item });
					}
				}
			}

			// result
			Ok(Some(item))
		} else {
			Ok(None)
		}
	}

	/// Store active items and return the root link.
	pub async fn store(&mut self) -> Result<OptionLink<Root<K, V>>, StorageError> {
		// get root
		let mut root = match self.root().await? {
			Some(root) => root,
			None => {
				// collection empty?
				if self.active.is_empty() {
					return Ok(OptionLink::none());
				}

				// new root
				Root { levels: Default::default(), active: Default::default(), settings: self.settings.clone() }
			},
		};

		// store active
		root.active = self.store_entries(stream::iter(self.active.iter()).map(Ok)).await?;

		// store root
		self.root = self.storage.set_value(&root).await?.into();

		// result
		Ok(self.root)
	}

	/// Flush active in memory entries as a new run.
	async fn flush_active(&mut self) -> Result<usize, StorageError> {
		// validate
		if self.active.is_empty() {
			return Ok(0);
		}
		let min_key = if let Some((min_key, _)) = self.active.first_key_value() {
			min_key.clone()
		} else {
			return Ok(0);
		};
		let max_key = if let Some((max_key, _)) = self.active.last_key_value() {
			max_key.clone()
		} else {
			return Ok(0);
		};
		if min_key == max_key {
			return Ok(0);
		}

		// run
		let run = if let Some(run) = self
			.store_run(
				stream::iter(self.active.iter()).map(|(key, value)| Ok((Cow::Borrowed(key), Cow::Borrowed(value)))),
			)
			.await?
		{
			run
		} else {
			return Ok(0);
		};

		// get/create root
		let mut root = match self.root().await? {
			Some(root) => root,
			None => Root { levels: Default::default(), active: Default::default(), settings: self.settings.clone() },
		};

		// add as first run to level 0
		//  note: this currently loads all run "metadata" entries into memory however that should be ok because we dont
		//   expect the run numbers to be very large.
		let mut level0 = match root.levels.get(0) {
			Some(level_link) => self.storage.get_value(level_link).await?,
			None => Level { runs: Default::default() },
		};
		let mut runs = NodeStream::from_link(self.storage.clone(), level0.runs)
			.try_collect::<VecDeque<_>>()
			.await?;
		runs.push_front(run);
		let runs_count = runs.len();
		level0.runs = self.store_entries(stream::iter(runs.iter()).map(Ok)).await?;
		let mut next_level0_link = self.storage.set_value(&level0).await?;

		// store root with next level0
		match root.levels.get_mut(0) {
			Some(level) => swap(level, &mut next_level0_link),
			None => root.levels.insert(0, next_level0_link),
		}
		self.root = self.storage.set_value(&root).await?.into();

		// cleanup when everything has succedded
		self.active.clear();

		// compact?
		if runs_count > self.settings.max_run_count as usize {
			self.compact_level(0).await?;
			// TODO: compact next level
		}

		// result
		Ok(0)
	}

	/// Store `entries` as DAG and return the root [`cid::Cid`].
	async fn store_entries<T, O>(
		&self,
		entries: impl Stream<Item = Result<T, StorageError>>,
	) -> Result<OptionLink<Node<O>>, StorageError>
	where
		T: Clone + Serialize,
	{
		let mut node_builder = NodeBuilder::<_, S::StoreParams>::new(
			self.settings
				.max_node_entries
				.try_into()
				.map_err(|e: TryFromIntError| StorageError::InvalidArgument(e.into()))?,
			Default::default(),
		);
		pin_mut!(entries);
		while let Some(entry) = entries.try_next().await? {
			node_builder.push(entry).map_err(|e| StorageError::InvalidArgument(e.into()))?;
			for block in node_builder.take_blocks() {
				self.storage.set(block).await?;
			}
		}
		let (root, blocks) = node_builder
			.into_blocks()
			.map_err(|e| StorageError::InvalidArgument(e.into()))?;
		for block in blocks.into_iter() {
			self.storage.set(block).await?;
		}

		// result
		Ok(root.into())
	}

	/// Store a new run to storage composed of `entries`.
	async fn store_run<'a>(
		&self,
		entries: impl Stream<Item = Result<(Cow<'_, K>, Cow<'_, Value<V>>), StorageError>>,
	) -> Result<Option<Run<K, V>>, StorageError> {
		// entries
		let mut min_key: Option<K> = None;
		let mut max_key: Option<K> = None;
		let entries_link = self
			.store_entries(entries.inspect_ok(|(key, _)| {
				if min_key.is_none() || min_key.as_ref().cmp(&Some(key)) == Ordering::Less {
					min_key = Some(key.as_ref().clone());
				}
				if max_key.is_none() || max_key.as_ref().cmp(&Some(key)) == Ordering::Greater {
					max_key = Some(key.as_ref().clone());
				}
			}))
			.await?;
		let (min_key, max_key) = match (min_key, max_key) {
			(Some(min_key), Some(max_key)) => (min_key, max_key),
			_ => return Ok(None),
		};
		let next_run = Run {
			entries: entries_link,
			bloom: vec![], // TODO: bloom
			min_key,
			max_key,
		};
		Ok(Some(next_run))
	}

	/// Compact `level` to next level.
	async fn compact_level(&mut self, level_index: usize) -> Result<(), StorageError> {
		let next_level_index = level_index + 1;

		// root
		let mut root = if let Some(root) = self.root().await? { root } else { return Ok(()) };

		// level
		let mut level = if let Some(level_link) = root.levels.get(level_index) {
			self.storage.get_value(level_link).await?
		} else {
			return Ok(());
		};

		// next level
		let mut next_level = if let Some(level_link) = root.levels.get(next_level_index) {
			self.storage.get_value(level_link).await?
		} else {
			Level { runs: OptionLink::none() }
		};

		// find runs to compact
		let mut runs = Vec::new();
		{
			let levels_and_runs = self.levels_and_runs();
			pin_mut!(levels_and_runs);
			let mut current_global_level_index = 0;
			while let Some(level_or_run) = levels_and_runs.try_next().await? {
				match level_or_run {
					Either::Left((global_level_index, _level)) => {
						current_global_level_index = global_level_index;
						// stop when hit older levels
						if current_global_level_index > next_level_index {
							break;
						}
					},
					Either::Right((global_run_index, run)) => {
						if current_global_level_index == level_index {
							// we use all runs from the level
							runs.push((current_global_level_index, global_run_index, run));
						} else if current_global_level_index == next_level_index {
							// test overlap in next level
							if runs
								.iter()
								.any(|(_, _, r)| r.min_key <= run.max_key && run.min_key <= r.max_key)
							{
								runs.push((current_global_level_index, global_run_index, run));
							}
						}
					},
				}
			}
		}
		if runs.is_empty() {
			return Ok(());
		}

		// runs
		let entries = self.create_stream(Some(runs.iter().map(|(_, global_run_index, _)| *global_run_index).collect()));
		let run = self.store_run(entries.map_ok(|(k, v)| (Cow::Owned(k), Cow::Owned(v)))).await?;
		let run = if let Some(run) = run {
			run
		} else {
			return Ok(());
		};

		// replace runs
		{
			let mut level_runs = NodeStream::from_link(self.storage.clone(), level.runs)
				.try_collect::<Vec<_>>()
				.await?;
			let mut next_level_runs = NodeStream::from_link(self.storage.clone(), next_level.runs)
				.try_collect::<Vec<_>>()
				.await?;

			// remove old
			for (global_level_index, global_run_index, _) in runs.iter().rev() {
				let local_run_index = *global_run_index - *global_level_index;
				if *global_level_index == level_index && level_runs.get(local_run_index).is_some() {
					level_runs.remove(*global_run_index - *global_level_index);
				}
				if *global_level_index == next_level_index && next_level_runs.get(local_run_index).is_some() {
					next_level_runs.remove(*global_run_index - *global_level_index);
				}
			}

			// insert new
			next_level_runs.insert(0, run);

			// store runs
			level.runs = self.store_entries(stream::iter(level_runs).map(Ok)).await?;
			next_level.runs = self.store_entries(stream::iter(next_level_runs).map(Ok)).await?;
		}

		// replace levels
		for (i, l) in [(level_index, level), (next_level_index, next_level)] {
			let mut link = self.storage.set_value(&l).await?;
			match root.levels.get_mut(i) {
				Some(level) => swap(level, &mut link),
				None => root.levels.insert(i, link),
			}
		}

		// replace root
		self.root = self.storage.set_value(&root).await?.into();

		// result
		Ok(())
	}

	/// Root.
	async fn root(&self) -> Result<Option<Root<K, V>>, StorageError> {
		Ok(self.storage.get_value_or_none(&self.root).await?)
	}

	/// All active levels.
	fn levels(&self) -> impl Stream<Item = Result<Level<K, V>, StorageError>> + '_ {
		async_stream::try_stream! {
			if let Some(root) = self.root().await? {
				for level_link in root.levels.iter() {
					let level = self.storage.get_value(level_link).await?;
					yield level;
				}
			}
		}
	}

	/// All active runs.
	/// Sorted by newest to oldest run.
	fn runs(&self) -> impl Stream<Item = Result<Run<K, V>, StorageError>> + '_ {
		async_stream::try_stream! {
			let levels = self.levels();
			for await level in levels {
				let level = level?;
				let runs = NodeStream::from_link(self.storage.clone(), level.runs);
				for await run in runs {
					yield run?;
				}
			}
		}
	}

	/// All active levels and runs with its (current) global index.
	/// Sorted by newest to oldest run.
	fn levels_and_runs(
		&self,
	) -> impl Stream<Item = Result<Either<(usize, Level<K, V>), (usize, Run<K, V>)>, StorageError>> + '_ {
		async_stream::try_stream! {
			let mut global_level_index = 0;
			let mut global_run_index = 0;
			let levels = self.levels();
			for await level in levels {
				let level = level?;
				let runs = NodeStream::from_link(self.storage.clone(), level.runs);
				yield Either::Left((global_level_index, level));
				for await run in runs {
					yield Either::Right((global_run_index, run?));
					global_run_index += 1;
				}
				global_level_index += 1;
			}
		}
	}
}

#[derive(Debug)]
struct TreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	run: Option<usize>,
	item: (K, Value<V>),
}
impl<K, V> PartialEq for TreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	fn eq(&self, other: &Self) -> bool {
		self.item.0 == other.item.0
	}
}
impl<K, V> Eq for TreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
}
impl<K, V> PartialOrd for TreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(&other))
	}
}
impl<K, V> Ord for TreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// invert the result as the binaryheap always returns the max item first
		match self.item.0.cmp(&other.item.0) {
			Ordering::Less => Ordering::Greater,
			Ordering::Greater => Ordering::Less,
			Ordering::Equal => {
				// sort by run index so the most recent run item is the last
				other.run.cmp(&self.run)
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::Tree;
	use crate::{Block, BlockStorage, DefaultParams, StorageError};
	use anyhow::anyhow;
	use async_trait::async_trait;
	use cid::Cid;
	use futures::{lock::Mutex, TryStreamExt};
	use std::{collections::BTreeMap, sync::Arc};

	#[derive(Debug, Default, Clone)]
	struct TestStorage {
		items: Arc<Mutex<BTreeMap<Cid, Block<DefaultParams>>>>,
	}
	#[async_trait]
	impl BlockStorage for TestStorage {
		type StoreParams = DefaultParams;

		async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
			self.items
				.lock()
				.await
				.get(cid)
				.ok_or_else(|| StorageError::NotFound(*cid, anyhow!("No record")))
				.cloned()
		}
		async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
			let cid = *block.cid();
			self.items.lock().await.insert(cid, block);
			Ok(cid)
		}
		async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
			self.items.lock().await.remove(cid);
			Ok(())
		}
	}

	#[tokio::test]
	async fn smoke() {
		let storage = TestStorage::default();
		let mut tree = Tree::new(storage.clone(), Default::default());
		tree.insert("hello".to_owned(), "world".to_owned()).await.unwrap();
		tree.insert("1".to_owned(), "2".to_owned()).await.unwrap();
		tree.insert("3".to_owned(), "4".to_owned()).await.unwrap();
		assert_eq!(
			tree.stream().try_collect::<Vec<_>>().await.unwrap(),
			vec![
				("1".to_owned(), "2".to_owned()),
				("3".to_owned(), "4".to_owned()),
				("hello".to_owned(), "world".to_owned())
			]
		);

		// reload
		let root = tree.store().await.unwrap().unwrap();
		let tree2 = Tree::load(storage.clone(), root).await.unwrap();
		assert_eq!(
			tree2.stream().try_collect::<Vec<_>>().await.unwrap(),
			vec![
				("1".to_owned(), "2".to_owned()),
				("3".to_owned(), "4".to_owned()),
				("hello".to_owned(), "world".to_owned())
			]
		);
	}
}
