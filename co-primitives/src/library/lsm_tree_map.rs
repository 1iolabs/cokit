use super::node_builder::NodeReader;
use crate::{
	Block, BlockSerializer, BlockStorage, BlockStorageExt, Link, Node, NodeBuilder, NodeBuilderError, NodeSerializer,
	NodeStream, OptionLink, StorageError, StoreParams,
};
use anyhow::anyhow;
use bloomfilter::Bloom;
use cid::Cid;
use either::Either;
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
	fmt::Debug,
	future::ready,
	hash::Hash,
	marker::PhantomData,
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
	#[serde(rename = "l", default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
	pub levels: Vec<Link<Level<K, V>>>,

	/// Active "in memory data".
	#[serde(rename = "a", default = "OptionLink::default", skip_serializing_if = "OptionLink::is_none")]
	pub active: OptionLink<Node<(K, Value<V>)>>,

	/// Tree settings.
	#[serde(
		rename = "s",
		default = "LsmTreeMapSettings::default",
		skip_serializing_if = "LsmTreeMapSettings::is_default"
	)]
	pub settings: LsmTreeMapSettings,
}

/// LSM Tree Level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Runs (SSTable) in the level.
	#[serde(rename = "r", default = "OptionLink::default", skip_serializing_if = "OptionLink::is_none")]
	pub runs: OptionLink<Node<Run<K, V>>>,
}

/// LSM Tree Run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Entries Root DAG Node.
	#[serde(rename = "e")]
	pub entries: OptionLink<RunNode<K, V>>,

	/// Count of entries.
	#[serde(rename = "s")]
	pub size: u64,

	/// Key Bloom Filter.
	#[serde(rename = "i")]
	pub bloom: BloomFilter,

	/// The lowest key.
	#[serde(rename = "l")]
	pub min_key: K,

	/// The highest key.
	#[serde(rename = "h")]
	pub max_key: K,
}
impl<K, V> Run<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Tests if the run possibly contains an key.
	fn may_contains_key(&self, key: &K) -> bool {
		key >= &self.min_key && key <= &self.max_key && self.bloom.may_contains_key(key)
	}
}

/// Bloom Filter Implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BloomFilter {
	/// `bloomfilter = "3"`
	#[serde(rename = "b", with = "serde_bytes")]
	Bloomfilter(Vec<u8>),
}
impl BloomFilter {
	/// Check if the bloom filter possibly contains an key.
	pub fn may_contains_key<K: Hash + Ord + Clone + Send + Sync + 'static>(&self, key: &K) -> bool {
		match self {
			BloomFilter::Bloomfilter(data) => {
				if let Ok(bloom) = Bloom::from_slice(&data) {
					bloom.check(key)
				} else {
					true
				}
			},
		}
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
pub enum Value<V>
where
	V: Clone + 'static,
{
	#[serde(rename = "v")]
	Value(V),
	#[serde(rename = "t")]
	Tombstone,
}
impl<V> PartialEq for Value<V>
where
	V: PartialEq + Clone + 'static,
{
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Tombstone, Self::Tombstone) => true,
			(Self::Value(a), Self::Value(b)) => a == b,
			_ => false,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunNode<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// A node which contains sorted children nodes.
	#[serde(rename = "n")]
	Node {
		/// The node.
		#[serde(rename = "n")]
		nodes: Vec<Link<Self>>,

		/// The lowest key.
		#[serde(rename = "l")]
		min_key: K,

		/// The highest key.
		#[serde(rename = "h")]
		max_key: K,
	},

	/// A leaf node which contains items sorted by key.
	#[serde(rename = "l")]
	Leaf(BTreeMap<K, Value<V>>),
}
impl<K, V> RunNode<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Tests if the run possibly contains an key.
	fn may_contains_key(&self, key: &K) -> bool {
		match self {
			RunNode::Node { nodes: _, min_key, max_key } => key >= min_key && key <= max_key,
			RunNode::Leaf(items) => items.contains_key(key),
		}
	}
}
impl<K, V> NodeReader<(K, Value<V>)> for RunNode<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	type Filter = RunNodeFilter<K>;

	fn read(self, filter: &Self::Filter) -> Either<Vec<Cid>, Vec<(K, Value<V>)>> {
		match self {
			RunNode::Node { nodes, min_key, max_key } => {
				if filter.test(&min_key, &max_key) {
					// println!("- filter {:?} min: {:?} max: {:?}", filter, min_key, max_key);
					Either::Left(Vec::new())
				} else {
					// println!("+ use    {:?} min: {:?} max: {:?}", filter, min_key, max_key);
					Either::Left(nodes.into_iter().map(Into::into).collect())
				}
			},
			RunNode::Leaf(items) => Either::Right(items.into_iter().collect()),
		}
	}
}

#[derive(Debug, Default)]
pub enum RunNodeFilter<K> {
	#[default]
	None,
	Max(K),
	Min(K),
}
impl<K> RunNodeFilter<K> {
	pub fn min(key: Option<K>) -> Self {
		match key {
			None => Self::None,
			Some(key) => Self::Min(key),
		}
	}

	pub fn max(key: Option<K>) -> Self {
		match key {
			None => Self::None,
			Some(key) => Self::Max(key),
		}
	}

	/// Test if min_key .. max_key should be skipped
	pub fn test(&self, min_key: &K, max_key: &K) -> bool
	where
		K: Ord,
	{
		match self {
			RunNodeFilter::Min(filter_min_key) => min_key > filter_min_key,
			RunNodeFilter::Max(filter_max_key) => max_key < filter_max_key,
			RunNodeFilter::None => false,
		}
	}
}

/// Serializer which records min/max keys while serialize blocks.
/// This metadata is written to the upper level blocks for fast retrival of values.
/// Note: This only works if all previous nodes are serialized using the same serialize instance.
///  But as we always wrote full immutable runs this should be ok.
#[derive(Debug)]
pub struct RunNodeSerializer<K, V> {
	_d: PhantomData<(K, V)>,
	pending: BTreeMap<Cid, (K, K)>,
}
impl<K, V> RunNodeSerializer<K, V> {
	pub fn new() -> Self {
		Self { _d: Default::default(), pending: Default::default() }
	}
}
impl<K, V, P> NodeSerializer<RunNode<K, V>, (K, Value<V>), P> for RunNodeSerializer<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	P: StoreParams,
{
	fn nodes(&mut self, nodes: Vec<Link<RunNode<K, V>>>) -> Result<RunNode<K, V>, NodeBuilderError> {
		let mut min_key = None;
		let mut max_key = None;
		for node in nodes.iter() {
			if let Some((node_min_key, node_max_key)) = self.pending.remove(node.cid()) {
				if min_key.is_none() || Some(&node_min_key).cmp(&min_key.as_ref()) == Ordering::Less {
					min_key = Some(node_min_key);
				}
				if max_key.is_none() || Some(&node_max_key).cmp(&max_key.as_ref()) == Ordering::Greater {
					max_key = Some(node_max_key);
				}
			}
		}
		Ok(RunNode::Node {
			nodes,
			min_key: min_key.ok_or(NodeBuilderError::InvalidArgument(anyhow!("Unable to determine min key")))?,
			max_key: max_key.ok_or(NodeBuilderError::InvalidArgument(anyhow!("Unable to determine max key")))?,
		})
	}

	fn leaf(&mut self, entries: Vec<(K, Value<V>)>) -> Result<RunNode<K, V>, NodeBuilderError> {
		Ok(RunNode::Leaf(entries.into_iter().collect()))
	}

	fn serialize(&mut self, node: RunNode<K, V>) -> Result<Block<P>, NodeBuilderError> {
		let block = BlockSerializer::new()
			.serialize(&node)
			.map_err(|err| NodeBuilderError::Encoding(err.into()))?;

		// record min/max for faster insert
		match &node {
			RunNode::Node { nodes: _, min_key, max_key } => {
				self.pending.insert(*block.cid(), (min_key.clone(), max_key.clone()));
			},
			RunNode::Leaf(items) => {
				if let (Some((first, _)), Some((last, _))) = (items.first_key_value(), items.last_key_value()) {
					self.pending.insert(*block.cid(), (first.clone(), last.clone()));
				}
			},
		}

		// result
		Ok(block)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LsmTreeMapSettings {
	/// Max entries (K/V pairs) count in a leaf node.
	#[serde(
		rename = "n",
		default = "LsmTreeMapSettings::default_max_node_entries",
		skip_serializing_if = "LsmTreeMapSettings::is_default_max_node_entries"
	)]
	pub max_node_entries: u64,

	/// Limits entries (K/V pairs) in active (in memory) run.
	/// Overruning this limit will cause a new L0 run gets created.
	#[serde(
		rename = "a",
		default = "LsmTreeMapSettings::default_max_active_entries",
		skip_serializing_if = "LsmTreeMapSettings::is_default_max_active_entries"
	)]
	pub max_active_entries: u64,

	/// Limits runs in a level.
	/// Overruning this limit will cause a compaction to next level.
	#[serde(
		rename = "r",
		default = "LsmTreeMapSettings::default_max_run_count",
		skip_serializing_if = "LsmTreeMapSettings::is_default_max_run_count"
	)]
	pub max_run_count: u64,
}
impl LsmTreeMapSettings {
	fn default_max_node_entries() -> u64 {
		2u64.checked_pow(8).unwrap() // 256
	}

	fn default_max_active_entries() -> u64 {
		2u64.checked_pow(14).unwrap() // 16k
	}

	fn default_max_run_count() -> u64 {
		2u64.checked_pow(4).unwrap() // 16
	}

	fn is_default_max_node_entries(value: &u64) -> bool {
		*value == Self::default_max_node_entries()
	}

	fn is_default_max_active_entries(value: &u64) -> bool {
		*value == Self::default_max_active_entries()
	}

	fn is_default_max_run_count(value: &u64) -> bool {
		*value == Self::default_max_run_count()
	}

	/// Wherter this are default settings.
	pub fn is_default(&self) -> bool {
		self == &Self::default()
	}
}
impl Default for LsmTreeMapSettings {
	fn default() -> Self {
		Self {
			max_node_entries: Self::default_max_node_entries(),
			max_active_entries: Self::default_max_active_entries(),
			max_run_count: Self::default_max_run_count(),
		}
	}
}

/// LSM Tree Instance.
pub struct LsmTreeMap<S, K, V>
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
	settings: LsmTreeMapSettings,
	// level_cache: Arc<RwLock<BTreeMap<Cid, Level<K, V>>>>,
	// run_cache: Arc<RwLock<BTreeMap<Cid, Run<K, V>>>>,
}
impl<S, K, V> LsmTreeMap<S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Create new empty tree.
	pub fn new(storage: S, settings: LsmTreeMapSettings) -> Self {
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
		if self.active.len() >= self.settings.max_active_entries as usize {
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
		if self.active.len() >= self.settings.max_active_entries as usize {
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
				let runs = self.levels_and_runs();
				pin_mut!(runs);
				while let Some(item) = runs.try_next().await? {
					if let Some((_, run)) = item.right() {
						if run.may_contains_key(key) {
							// iterate (DAG) RunNode's from first to last skipping nodes which can not contain the
							// value
							let mut stack: VecDeque<Link<RunNode<K, V>>> = Default::default();
							if let Some(cid) = run.entries.link() {
								stack.push_back(cid);
							}
							while let Some(cid) = stack.pop_front() {
								let node = self.storage.get_value(&cid).await?;
								if node.may_contains_key(key) {
									match node {
										RunNode::Node { nodes, min_key: _, max_key: _ } => {
											stack.extend(nodes.into_iter());
										},
										RunNode::Leaf(mut items) => {
											if let Some(value) = items.remove(key) {
												return match value {
													Value::Value(value) => Ok(Some(value)),
													Value::Tombstone => Ok(None),
												};
											}
										},
									}
								}
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
		self.stream_query(None)
	}

	/// Stream tree entries.
	pub fn stream_query(&self, start_at: Option<K>) -> impl Stream<Item = Result<(K, V), StorageError>> + '_ {
		self.create_stream(None, start_at).try_filter_map(|item| {
			ready(Ok(match item.1 {
				Value::Value(value) => Some((item.0, value)),
				Value::Tombstone => None,
			}))
		})
	}

	/// Stream all tree entries in reverse order.
	pub fn reverse_stream(&self) -> impl Stream<Item = Result<(K, V), StorageError>> + '_ {
		self.reverse_stream_query(None)
	}

	/// Stream tree entries in reverse order.
	pub fn reverse_stream_query(&self, start_at: Option<K>) -> impl Stream<Item = Result<(K, V), StorageError>> + '_ {
		self.create_reverse_stream(None, start_at).try_filter_map(|item| {
			ready(Ok(match item.1 {
				Value::Value(value) => Some((item.0, value)),
				Value::Tombstone => None,
			}))
		})
	}

	/// Store active items and return the root link.
	///
	/// # Guarantees
	/// - The method detects if the collection is empty after this and accordingly returns [`OptionLink::none`].
	pub async fn store(&mut self) -> Result<OptionLink<Root<K, V>>, StorageError> {
		// get root
		let mut root = match self.root().await? {
			Some(root) => {
				// collection empty (no items or only tombstoned items)?
				if self.is_empty().await? {
					return Ok(OptionLink::none());
				}

				// root
				root
			},
			None => {
				// collection empty (no root and no active)?
				if self.active.is_empty() || self.is_empty().await? {
					return Ok(OptionLink::none());
				}

				// new root
				Root { levels: Default::default(), active: Default::default(), settings: self.settings.clone() }
			},
		};

		// store active
		root.active =
			store_node(&self.storage, self.settings.max_node_entries, stream::iter(self.active.iter()).map(Ok)).await?;

		// store root
		self.root = self.storage.set_value(&root).await?.into();

		// result
		Ok(self.root)
	}

	/// Tree stats.
	pub async fn stats(&self) -> Result<LsmTreeStats, StorageError> {
		Ok(self
			.levels_and_runs()
			.try_fold(
				LsmTreeStats { entries: 0, active_entries: self.active.len(), levels: 0, runs: 0 },
				|mut result, item| {
					match item {
						Either::Left(_) => result.levels += 1,
						Either::Right((_, run)) => {
							result.runs += 1;
							result.entries += run.size as usize;
						},
					}
					ready(Ok(result))
				},
			)
			.await?)
	}

	/// Whether the collection is empty.
	pub async fn is_empty(&self) -> Result<bool, StorageError> {
		Ok(self.min_key().await?.is_none())
	}

	/// Find the first (active - not tombstoned) key.
	pub async fn min_key(&self) -> Result<Option<K>, StorageError> {
		let stream = self.stream();
		pin_mut!(stream);
		let first = stream.try_next().await?;
		Ok(first.map(|(key, _)| key))
	}

	/// Find the last (active - not tombstoned) key.
	pub async fn max_key(&self) -> Result<Option<K>, StorageError> {
		let stream = self.reverse_stream();
		pin_mut!(stream);
		let first = stream.try_next().await?;
		Ok(first.map(|(key, _)| key))
	}

	/// Compact tree.
	///
	/// # Arguments
	/// - `flush_active` - Flush active "in memory" entries to L0.
	pub async fn compact(&mut self, flush_active: bool) -> Result<(), StorageError> {
		if flush_active {
			self.flush_active().await?;
		}
		self.compact_level(0, Some(self.settings.max_run_count as usize)).await?;
		Ok(())
	}

	/// Iterate on runs.
	///
	/// # Arguments
	/// - `only_run_indicies` - Only use specified runs specified by global run index.
	/// - `start_at` - Start streaming at (inclusive) key.
	fn create_stream(
		&self,
		only_run_indicies: Option<BTreeSet<usize>>,
		start_at: Option<K>,
	) -> impl Stream<Item = Result<(K, Value<V>), StorageError>> + Send + '_ {
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
			let mut runs: Vec<(usize, NodeStream<S, (K, Value<V>), RunNode<K, V>>)> = self
				.levels_and_runs()
				.try_filter_map(|item| ready(Ok(item.right())))
				.try_filter(|(index, _)| ready(match &only_run_indicies {
					Some(run_indicies) => run_indicies.contains(index),
					None => true,
				}))
				.map_ok(|(index, run)| (index, NodeStream::from_link(storage.clone(), run.entries).with_filter(RunNodeFilter::max(start_at.clone()))))
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

				// skip items before start at
				//  we need to filter overlaps as nodes which come before start_at will be skipped
				if let Some(start_at) = &start_at {
					if !(start_at <= &item.item.0) {
						continue;
					}
				}

				// yield values
				yield item.item;
			}
		}
		// TreeStream::new(self)
	}

	/// Iterate on runs.
	fn create_reverse_stream(
		&self,
		only_run_indicies: Option<BTreeSet<usize>>,
		start_at: Option<K>,
	) -> impl Stream<Item = Result<(K, Value<V>), StorageError>> + '_ {
		let storage = self.storage.clone();
		async_stream::try_stream! {
			// heap (max-sorted)
			let mut heap = match only_run_indicies {
				Some(_) => BinaryHeap::new(),
				None => BinaryHeap::<ReverseTreeStreamItem<K, V>>::from(
					self.active
						.clone()
						.into_iter()
						.map(|item| ReverseTreeStreamItem { run: None, item })
						.collect::<Vec<_>>(),
				),
			};

			// runs
			//  filter and pop first item of each run
			let mut runs: Vec<(usize, NodeStream<S, (K, Value<V>), RunNode<K, V>>)> = self
				.levels_and_runs()
				.try_filter_map(|item| ready(Ok(item.right())))
				.try_filter(|(index, _)| ready(match &only_run_indicies {
					Some(run_indicies) => run_indicies.contains(index),
					None => true,
				}))
				.map_ok(|(index, run)| (index, NodeStream::from_link(storage.clone(), run.entries).with_reverse().with_filter(RunNodeFilter::min(start_at.clone()))))
				.try_collect()
				.await?;
			for (run_index, run) in runs.iter_mut() {
				if let Some(item) = run.try_next().await? {
					heap.push(ReverseTreeStreamItem { run: Some(*run_index), item });
				}
			}

			// walk tree
			while let Some(item) = Self::pop_and_fetch_reverse(&mut heap, &mut runs).await? {
				// pop superseded keys
				//  we sort the ReverseNodeStream by key and run index so we receive the next key front the most recent run first
				while let Some(next_item) = heap.peek() {
					if next_item.item.0 == item.item.0 {
						Self::pop_and_fetch_reverse(&mut heap, &mut runs).await?;
					} else {
						break;
					}
				}

				// skip items after start at
				//  we need to filter overlaps as nodes which come after start_at will be skipped
				if let Some(start_at) = &start_at {
					if !(start_at >= &item.item.0) {
						continue;
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
		runs: &mut Vec<(usize, NodeStream<S, (K, Value<V>), RunNode<K, V>>)>,
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

	/// Pop item and continue to read the run.
	async fn pop_and_fetch_reverse(
		heap: &mut BinaryHeap<ReverseTreeStreamItem<K, V>>,
		runs: &mut Vec<(usize, NodeStream<S, (K, Value<V>), RunNode<K, V>>)>,
	) -> Result<Option<ReverseTreeStreamItem<K, V>>, StorageError> {
		if let Some(item) = heap.pop() {
			// fetch next
			//  every time we take an item from an run we fetch the next one
			//  so the heap can determine which are the next in sequence
			//  once a run runs out of items it will not be asked again as all run items poped
			if let Some(run_index) = item.run {
				if let Some((_, run)) = runs.get_mut(run_index) {
					if let Some(item) = run.try_next().await? {
						heap.push(ReverseTreeStreamItem { run: Some(run_index), item });
					}
				}
			}

			// result
			Ok(Some(item))
		} else {
			Ok(None)
		}
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
		//  TODO: remove clone (because of the hardcoded Value<V>?)
		let run = if let Some(run) = store_run(
			&self.storage,
			self.settings.max_node_entries,
			stream::iter(self.active.clone().into_iter()).map(Ok),
			self.active.len(),
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
		level0.runs =
			store_items(&self.storage, self.settings.max_node_entries, stream::iter(runs.iter()).map(Ok)).await?;
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
		if runs_count >= self.settings.max_run_count as usize {
			self.compact_level(0, Some(self.settings.max_run_count as usize)).await?;
		}

		// result
		Ok(0)
	}

	/// Compact `level` to next level.
	///
	/// # Arguments
	/// - `level_index` - The gloabl level index to compact
	/// - `cascade` - Cascade to next (`level_index + n`) levels when they react the specified run count
	async fn compact_level(&mut self, level_index: usize, cascade: Option<usize>) -> Result<(), StorageError> {
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
		let entries =
			self.create_stream(Some(runs.iter().map(|(_, global_run_index, _)| *global_run_index).collect()), None);
		let run = store_run(
			&self.storage,
			self.settings.max_node_entries,
			entries,
			runs.iter().fold(0, |result, (_, _, run)| result + run.size) as usize,
		)
		.await?;
		let run = if let Some(run) = run {
			run
		} else {
			return Ok(());
		};

		// replace runs
		let (level_run_count, next_level_run_count) = {
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
			let level_run_count = level_runs.len();
			let next_level_run_count = next_level_runs.len();
			level.runs =
				store_items(&self.storage, self.settings.max_node_entries, stream::iter(&level_runs).map(Ok)).await?;
			next_level.runs =
				store_items(&self.storage, self.settings.max_node_entries, stream::iter(&next_level_runs).map(Ok))
					.await?;

			// counts
			(level_run_count, next_level_run_count)
		};

		// replace levels
		for (i, l) in [(level_index, level), (next_level_index, next_level)] {
			let mut link = self.storage.set_value(&l).await?;
			match root.levels.get_mut(i) {
				Some(level) => swap(level, &mut link),
				None => root.levels.insert(i, link),
			}
		}

		// clear empty
		let next_level_index = if level_run_count == 0 {
			root.levels.remove(level_index);
			level_index
		} else {
			next_level_index
		};

		// replace root
		self.root = self.storage.set_value(&root).await?.into();

		// cascade
		if let Some(max_run_count) = cascade {
			if next_level_run_count >= max_run_count {
				Box::pin(self.compact_level(next_level_index, cascade)).await?;
			}
		}

		// result
		// Ok((next_level_index, next_level_run_count))
		Ok(())
	}

	/// Root.
	async fn root(&self) -> Result<Option<Root<K, V>>, StorageError> {
		Ok(self.storage.get_value_or_none(&self.root).await?)
	}

	/// All active levels and runs with its (current) global index.
	/// Sorted by newest to oldest run.
	fn levels_and_runs(
		&self,
	) -> impl Stream<Item = Result<Either<(usize, Level<K, V>), (usize, Run<K, V>)>, StorageError>> + Send + '_ {
		async_stream::try_stream! {
			let mut global_level_index = 0;
			let mut global_run_index = 0;
			if let Some(root) = self.root().await? {
				for level_link in root.levels.iter() {
					let level = self.storage.get_value(level_link).await?;
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
}

/// Store as DAG Node and return the root [`cid::Cid`].
async fn store_items<'a, S, T>(
	storage: &S,
	max_node_entries: u64,
	items: impl Stream<Item = Result<&'a T, StorageError>> + Send,
) -> Result<OptionLink<Node<T>>, StorageError>
where
	S: BlockStorage + Clone + 'static,
	T: Clone + Serialize + 'a,
{
	let mut node_builder = NodeBuilder::<_, S::StoreParams>::new(
		max_node_entries
			.try_into()
			.map_err(|e: TryFromIntError| StorageError::InvalidArgument(e.into()))?,
		Default::default(),
	);
	pin_mut!(items);
	while let Some(item) = items.try_next().await? {
		node_builder.push(item).map_err(|e| StorageError::InvalidArgument(e.into()))?;
		for block in node_builder.take_blocks() {
			storage.set(block).await?;
		}
	}
	let (root, blocks) = node_builder
		.into_blocks()
		.map_err(|e| StorageError::InvalidArgument(e.into()))?;
	for block in blocks.into_iter() {
		storage.set(block).await?;
	}

	// result
	Ok(root.cid().into())
}

/// Store `entries` as DAG Node and return the root [`cid::Cid`].
async fn store_node<S, K, V>(
	storage: &S,
	max_node_entries: u64,
	entries: impl Stream<Item = Result<(&K, &Value<V>), StorageError>>,
) -> Result<OptionLink<Node<(K, Value<V>)>>, StorageError>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	let mut node_builder = NodeBuilder::<_, S::StoreParams>::new(
		max_node_entries
			.try_into()
			.map_err(|e: TryFromIntError| StorageError::InvalidArgument(e.into()))?,
		Default::default(),
	);
	pin_mut!(entries);
	while let Some(item) = entries.try_next().await? {
		node_builder.push(item).map_err(|e| StorageError::InvalidArgument(e.into()))?;
		for block in node_builder.take_blocks() {
			storage.set(block).await?;
		}
	}
	let (root, blocks) = node_builder
		.into_blocks()
		.map_err(|e| StorageError::InvalidArgument(e.into()))?;
	for block in blocks.into_iter() {
		storage.set(block).await?;
	}

	// result
	Ok(root.cid().into())
}

/// Store `entries` as DAG and return the root [`cid::Cid`].
async fn store_run_node<S, K, V>(
	storage: &S,
	max_node_entries: u64,
	entries: impl Stream<Item = Result<(K, Value<V>), StorageError>>,
) -> Result<OptionLink<RunNode<K, V>>, StorageError>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	let mut node_builder = NodeBuilder::<(K, Value<V>), S::StoreParams, RunNode<K, V>, RunNodeSerializer<K, V>>::new(
		max_node_entries
			.try_into()
			.map_err(|e: TryFromIntError| StorageError::InvalidArgument(e.into()))?,
		RunNodeSerializer::new(),
	);
	pin_mut!(entries);
	while let Some(item) = entries.try_next().await? {
		node_builder.push(item).map_err(|e| StorageError::InvalidArgument(e.into()))?;
		for block in node_builder.take_blocks() {
			storage.set(block).await?;
		}
	}
	let (root, blocks) = node_builder
		.into_blocks()
		.map_err(|e| StorageError::InvalidArgument(e.into()))?;
	for block in blocks.into_iter() {
		storage.set(block).await?;
	}

	// result
	Ok(root.into())
}

/// Store a new run to storage composed of `entries`.
async fn store_run<S, K, V>(
	storage: &S,
	max_node_entries: u64,
	entries: impl Stream<Item = Result<(K, Value<V>), StorageError>>,
	entries_size_hint: usize,
) -> Result<Option<Run<K, V>>, StorageError>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	// entries
	let mut min_key: Option<K> = None;
	let mut max_key: Option<K> = None;
	let mut size = 0;
	let mut bloom = Bloom::<K>::new_for_fp_rate_with_seed(entries_size_hint, 0.001, &[0; 32])
		.map_err(|e| StorageError::InvalidArgument(anyhow!("bloomfilter: {}", e)))?;
	let entries_link = store_run_node(
		storage,
		max_node_entries,
		entries.inspect_ok(|(key, _)| {
			if min_key.is_none() || Some(key).cmp(&min_key.as_ref()) == Ordering::Less {
				min_key = Some(key.clone());
			}
			if max_key.is_none() || Some(key).cmp(&max_key.as_ref()) == Ordering::Greater {
				max_key = Some(key.clone());
			}
			size += 1;
			bloom.set(key);
		}),
	)
	.await?;
	let (min_key, max_key) = match (min_key, max_key) {
		(Some(min_key), Some(max_key)) => (min_key, max_key),
		_ => return Ok(None),
	};
	let next_run =
		Run { entries: entries_link, size, bloom: BloomFilter::Bloomfilter(bloom.to_bytes()), min_key, max_key };
	Ok(Some(next_run))
}

/// Tree stats.
#[derive(Debug, Clone, PartialEq)]
pub struct LsmTreeStats {
	/// Approx. entries in tree (upper bound).
	pub entries: usize,

	/// Active "in memory" entries.
	pub active_entries: usize,

	/// Levels in tree.
	pub levels: usize,

	/// Runs in tree.
	pub runs: usize,
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

#[derive(Debug)]
struct ReverseTreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	run: Option<usize>,
	item: (K, Value<V>),
}
impl<K, V> PartialEq for ReverseTreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	fn eq(&self, other: &Self) -> bool {
		self.item.0 == other.item.0
	}
}
impl<K, V> Eq for ReverseTreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
}
impl<K, V> PartialOrd for ReverseTreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(&other))
	}
}
impl<K, V> Ord for ReverseTreeStreamItem<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match self.item.0.cmp(&other.item.0) {
			Ordering::Less => Ordering::Less,
			Ordering::Greater => Ordering::Greater,
			Ordering::Equal => {
				// sort by run index so the most recent run item is the last
				other.run.cmp(&self.run)
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{LsmTreeMap, Value};
	use crate::{
		from_cbor,
		library::{lsm_tree_map::LsmTreeStats, test::TestStorage},
		to_cbor, LsmTreeMapSettings,
	};
	use futures::TryStreamExt;

	#[tokio::test]
	async fn smoke() {
		let storage = TestStorage::default();
		let mut tree = LsmTreeMap::new(storage.clone(), Default::default());
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
		let tree2 = LsmTreeMap::load(storage.clone(), root).await.unwrap();
		assert_eq!(
			tree2.stream().try_collect::<Vec<_>>().await.unwrap(),
			vec![
				("1".to_owned(), "2".to_owned()),
				("3".to_owned(), "4".to_owned()),
				("hello".to_owned(), "world".to_owned())
			]
		);
	}

	#[tokio::test]
	async fn test_get() {
		let storage = TestStorage::default();
		let settings = LsmTreeMapSettings { max_node_entries: 32, max_active_entries: 2, max_run_count: 2 };
		let mut tree = LsmTreeMap::new(storage.clone(), settings);
		tree.insert(1, 100).await.unwrap();
		tree.insert(2, 200).await.unwrap();
		tree.insert(3, 300).await.unwrap();
		assert_eq!(tree.get(&1).await.unwrap(), Some(100));
		assert_eq!(tree.get(&2).await.unwrap(), Some(200));
		assert_eq!(tree.get(&3).await.unwrap(), Some(300));
	}

	#[tokio::test]
	async fn test_compact() {
		let storage = TestStorage::default();
		let settings = LsmTreeMapSettings { max_node_entries: 32, max_active_entries: 2, max_run_count: 2 };
		let mut tree = LsmTreeMap::new(storage.clone(), settings);
		for i in 0..10 {
			tree.insert(i, i).await.unwrap();
		}
		assert_eq!(
			tree.stream().try_collect::<Vec<_>>().await.unwrap(),
			vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (7, 7), (8, 8), (9, 9),]
		);
		let stats = tree.stats().await.unwrap();
		assert_eq!(stats, LsmTreeStats { active_entries: 0, entries: 10, levels: 1, runs: 1 });
		// somethinf like this should happen:
		// flush: 2
		// store run: 2
		// flush: 2
		// store run: 2
		// store run: 4
		// compact: from 0 to 1 with size 4
		// drop level 0
		// flush: 2
		// store run: 2
		// store run: 6
		// compact: from 0 to 1 with size 6
		// drop level 0
		// flush: 2
		// store run: 2
		// store run: 8
		// compact: from 0 to 1 with size 8
		// drop level 0
		// flush: 2
		// store run: 2
		// store run: 10
		// compact: from 0 to 1 with size 10
		// drop level 0
	}

	#[tokio::test]
	async fn test_stream() {
		let storage = TestStorage::default();
		let settings = LsmTreeMapSettings { max_node_entries: 32, max_active_entries: 2, max_run_count: 2 };
		let mut tree = LsmTreeMap::new(storage.clone(), settings);
		tree.insert(0, 0).await.unwrap();
		tree.insert(1, 1).await.unwrap();
		tree.insert(2, 2).await.unwrap();
		tree.insert(3, 3).await.unwrap();
		tree.remove(3).await.unwrap();
		tree.insert(3, 30).await.unwrap();
		tree.remove(3).await.unwrap();
		tree.insert(2, 20).await.unwrap();
		tree.flush_active().await.unwrap();
		assert_eq!(tree.stream().try_collect::<Vec<_>>().await.unwrap(), vec![(0, 0), (1, 1), (2, 20),]);
		assert_eq!(tree.reverse_stream().try_collect::<Vec<_>>().await.unwrap(), vec![(2, 20), (1, 1), (0, 0),]);
	}

	#[tokio::test]
	async fn test_stream_query() {
		let storage = TestStorage::default();
		let settings = LsmTreeMapSettings { max_node_entries: 2, max_active_entries: 2, max_run_count: 2 };
		let mut tree = LsmTreeMap::new(storage.clone(), settings);
		for i in 0..10 {
			tree.insert(i, i).await.unwrap();
		}
		assert_eq!(
			tree.stream_query(Some(5)).try_collect::<Vec<_>>().await.unwrap(),
			vec![(5, 5), (6, 6), (7, 7), (8, 8), (9, 9)]
		);
		assert_eq!(
			tree.reverse_stream_query(Some(5)).try_collect::<Vec<_>>().await.unwrap(),
			vec![(5, 5), (4, 4), (3, 3), (2, 2), (1, 1), (0, 0)]
		);
	}

	#[tokio::test]
	async fn test_store_empty() {
		for count in [1, 10] {
			let storage = TestStorage::default();
			let settings = LsmTreeMapSettings { max_node_entries: 2, max_active_entries: 2, max_run_count: 2 };
			let mut tree = LsmTreeMap::new(storage.clone(), settings);
			for i in 0..count {
				tree.insert(i, i).await.unwrap();
			}
			for i in 0..count {
				tree.remove(i).await.unwrap();
			}
			assert!(tree.store().await.unwrap().is_none());
		}
	}

	#[test]
	fn test_settings_default() {
		assert_eq!(LsmTreeMapSettings::default().max_node_entries, 256);
		assert_eq!(LsmTreeMapSettings::default().max_active_entries, 16384);
		assert_eq!(LsmTreeMapSettings::default().max_run_count, 16);
		assert!(LsmTreeMapSettings::default().is_default());
		let mut not_default = LsmTreeMapSettings::default();
		not_default.max_node_entries += 1;
		assert!(!not_default.is_default());
	}

	#[test]
	fn test_serialize_value() {
		let empty_v = Value::<()>::Value(());
		let v = Value::<u8>::Value(0);
		let t = Value::<u8>::Tombstone;

		// cbor
		let v_cbor = to_cbor(&v).unwrap();
		let empty_v_cbor = to_cbor(&empty_v).unwrap();
		let t_cbor = to_cbor(&t).unwrap();
		assert_eq!(from_cbor::<Value::<()>>(&empty_v_cbor).unwrap(), empty_v);
		assert_eq!(from_cbor::<Value::<u8>>(&v_cbor).unwrap(), v);
		assert_eq!(from_cbor::<Value::<u8>>(&t_cbor).unwrap(), t);
	}
}
