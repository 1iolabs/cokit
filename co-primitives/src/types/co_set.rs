use crate::{library::lsm_tree_map::Root, BlockStorage, LsmTreeMap, OptionLink, StorageError};
use futures::{Stream, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{future::Future, hash::Hash};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CoSet<K>(OptionLink<Root<K, SetValZST>>)
where
	K: Hash + Ord + Clone + Send + Sync + 'static;
impl<K> CoSet<K>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Whether this collection is empty.
	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	pub async fn contains<S>(&self, storage: &S, key: &K) -> Result<bool, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.open(storage).await?.contains(key).await
	}

	pub fn stream<S>(&self, storage: &S) -> impl Stream<Item = Result<K, StorageError>> + '_
	where
		S: BlockStorage + Clone + 'static,
	{
		let storage = storage.clone();
		async_stream::try_stream! {
			let transaction = self.open(&storage).await?;
			let stream = transaction.stream();
			for await item in stream {
				yield item?;
			}
		}
	}

	pub async fn insert<S>(&mut self, storage: &S, key: K) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.insert(key).await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	/// Remove key from set and return `true` if it was present.
	pub async fn remove<S>(&mut self, storage: &S, key: K) -> Result<bool, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.remove(key).await?;
		if result {
			self.commit(transaction).await?;
		}
		Ok(result)
	}
}
impl<K> CoSet<K>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	pub async fn open<S>(&self, storage: &S) -> Result<CoSetTransaction<S, K>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		Ok(CoSetTransaction {
			tree: match self.0.link() {
				Some(root) => LsmTreeMap::load(storage.clone(), root).await?,
				None => LsmTreeMap::new(storage.clone(), Default::default()),
			},
		})
	}

	/// Commit transaction to this map.
	pub async fn commit<S>(&mut self, mut transaction: CoSetTransaction<S, K>) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.0 = transaction.tree.store().await?;
		Ok(())
	}

	/// Open transaction and apply `update` and store it.
	pub async fn update<S, F, Fut>(&mut self, storage: &S, update: F) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
		F: FnOnce(CoSetTransaction<S, K>) -> Fut,
		Fut: Future<Output = Result<CoSetTransaction<S, K>, StorageError>>,
	{
		let transaction = self.open(storage).await?;
		let mut result = update(transaction).await?;
		self.0 = result.tree.store().await?;
		Ok(())
	}
}
impl<K> Default for CoSet<K>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self(Default::default())
	}
}

pub struct CoSetTransaction<S, K>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	tree: LsmTreeMap<S, K, SetValZST>,
}
impl<S, K> CoSetTransaction<S, K>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	pub async fn contains(&self, key: &K) -> Result<bool, StorageError> {
		self.tree.contains_key(key).await
	}

	pub fn stream(&self) -> impl Stream<Item = Result<K, StorageError>> + '_ {
		self.tree.stream().map_ok(|(key, _)| key)
	}

	pub async fn insert(&mut self, key: K) -> Result<(), StorageError> {
		self.tree.insert(key, SetValZST).await
	}

	/// Remove key from set and return `true` if it was present.
	pub async fn remove(&mut self, key: K) -> Result<bool, StorageError> {
		if let Some(_) = self.tree.get(&key).await? {
			self.tree.remove(key).await?;
			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Store as new CoSet
	pub async fn store(&mut self) -> Result<CoSet<K>, StorageError> {
		let link = self.tree.store().await?;
		Ok(CoSet(link))
	}
}

/// Zero-Sized Type (ZST) for internal `CoSet` values.
/// Used instead of `()` to differentiate between:
/// * `CoSet<T, ()>` (possible user-defined map)
/// * `CoSet<T, SetValZST>` (internal set representation)
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Default, Serialize, Deserialize)]
struct SetValZST;
