use super::lazy_transaction::Transactionable;
use crate::{
	library::lsm_tree_map::Root, AnyBlockStorage, LazyTransaction, LsmTreeMap, OptionLink, StorageError, Streamable,
};
use async_trait::async_trait;
use cid::Cid;
use futures::{stream::BoxStream, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, future::Future, hash::Hash};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CoSet<K>(OptionLink<Root<K, SetValZST>>)
where
	K: Hash + Ord + Clone + Send + Sync + 'static;
impl<K> CoSet<K>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Create collection from iterator.
	pub async fn from_iter<S>(storage: &S, iter: impl IntoIterator<Item = K>) -> Result<Self, StorageError>
	where
		S: AnyBlockStorage,
	{
		let mut transaction = Self::default().open(storage).await?;
		for key in iter.into_iter() {
			transaction.insert(key).await?;
		}
		transaction.store().await
	}

	/// Whether this collection is empty.
	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	pub async fn contains<S>(&self, storage: &S, key: &K) -> Result<bool, StorageError>
	where
		S: AnyBlockStorage,
	{
		self.open(storage).await?.contains(key).await
	}

	pub fn stream<S>(&self, storage: &S) -> impl Stream<Item = Result<K, StorageError>> + '_
	where
		S: AnyBlockStorage,
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
		S: AnyBlockStorage,
	{
		let mut transaction = self.open(storage).await?;
		transaction.insert(key).await?;
		self.commit(transaction).await?;
		Ok(())
	}

	/// Remove key from set and return `true` if it was present.
	pub async fn remove<S>(&mut self, storage: &S, key: K) -> Result<bool, StorageError>
	where
		S: AnyBlockStorage,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.remove(key).await?;
		if result {
			self.commit(transaction).await?;
		}
		Ok(result)
	}

	pub async fn open<S>(&self, storage: &S) -> Result<CoSetTransaction<S, K>, StorageError>
	where
		S: AnyBlockStorage,
	{
		Ok(CoSetTransaction {
			tree: match self.0.link() {
				Some(root) => LsmTreeMap::load(storage.clone(), root).await?,
				None => LsmTreeMap::new(storage.clone(), Default::default()),
			},
		})
	}

	pub async fn open_lazy<S>(&self, storage: &S) -> Result<LazyTransaction<S, Self>, StorageError>
	where
		S: AnyBlockStorage,
	{
		Ok(LazyTransaction::new(storage.clone(), self.clone()))
	}

	/// Commit transaction to this map.
	pub async fn commit<S>(&mut self, mut transaction: CoSetTransaction<S, K>) -> Result<(), StorageError>
	where
		S: AnyBlockStorage,
	{
		self.0 = transaction.tree.store().await?;
		Ok(())
	}

	/// Open transaction, apply `update` and store it.
	pub async fn with_transaction<S, F, Fut>(&mut self, storage: &S, update: F) -> Result<(), StorageError>
	where
		S: AnyBlockStorage,
		F: FnOnce(CoSetTransaction<S, K>) -> Fut + Send,
		Fut: Future<Output = Result<CoSetTransaction<S, K>, StorageError>> + Send,
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
impl<K> From<Option<Cid>> for CoSet<K>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
{
	fn from(value: Option<Cid>) -> Self {
		Self(value.into())
	}
}
impl<K> From<&CoSet<K>> for Option<Cid>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
{
	fn from(value: &CoSet<K>) -> Self {
		*value.0.cid()
	}
}
#[async_trait]
impl<S, K> Transactionable<S> for CoSet<K>
where
	S: AnyBlockStorage,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	type Transaction = CoSetTransaction<S, K>;

	async fn open(&self, storage: &S) -> Result<Self::Transaction, StorageError> {
		CoSet::open(self, storage).await
	}
}
impl<S, K> Streamable<S> for CoSet<K>
where
	S: AnyBlockStorage,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	type Item = Result<K, StorageError>;
	type Stream = BoxStream<'static, Self::Item>;

	fn stream(&self, storage: S) -> Self::Stream {
		let collection = self.clone();
		async_stream::try_stream! {
			let transaction = collection.open(&storage).await?;
			let stream = transaction.stream();
			for await item in stream {
				yield item?;
			}
		}
		.boxed()
	}
}

#[derive(Clone)]
pub struct CoSetTransaction<S, K>
where
	S: AnyBlockStorage,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	tree: LsmTreeMap<S, K, SetValZST>,
}
impl<S, K> CoSetTransaction<S, K>
where
	S: AnyBlockStorage,
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
		if (self.tree.get(&key).await?).is_some() {
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

#[cfg(test)]
mod tests {
	use crate::{library::test::TestStorage, CoSet};
	use futures::TryStreamExt;

	#[tokio::test]
	async fn smoke() {
		let storage = TestStorage::default();
		let mut set = CoSet::<i32>::default();
		let mut transaction = set.open(&storage).await.unwrap();
		transaction.insert(1).await.unwrap();
		transaction.insert(2).await.unwrap();
		set.commit(transaction).await.unwrap();
		assert_eq!(set.stream(&storage).try_collect::<Vec<i32>>().await.unwrap(), vec![1, 2]);
	}

	#[tokio::test]
	async fn test_remove() {
		let storage = TestStorage::default();
		let mut set = CoSet::<i32>::default();
		let mut transaction = set.open(&storage).await.unwrap();
		transaction.insert(1).await.unwrap();
		transaction.insert(2).await.unwrap();
		transaction.insert(3).await.unwrap();
		transaction.remove(1).await.unwrap();
		set.commit(transaction).await.unwrap();
		assert_eq!(set.stream(&storage).try_collect::<Vec<i32>>().await.unwrap(), vec![2, 3]);

		let mut transaction = set.open(&storage).await.unwrap();
		transaction.remove(3).await.unwrap();
		set.commit(transaction).await.unwrap();
		assert_eq!(set.stream(&storage).try_collect::<Vec<i32>>().await.unwrap(), vec![2]);
	}

	#[tokio::test]
	async fn test_remove_large() {
		let storage = TestStorage::default();
		let mut set = CoSet::<i32>::default();
		let mut transaction = set.open(&storage).await.unwrap();
		let range = 0..131072;
		for i in range.clone() {
			transaction.insert(i).await.unwrap();
		}
		set.commit(transaction).await.unwrap();
		let mut expect = range.collect::<Vec<i32>>();
		assert_eq!(set.stream(&storage).try_collect::<Vec<i32>>().await.unwrap(), expect);

		let mut transaction = set.open(&storage).await.unwrap();
		transaction.remove(10).await.unwrap();
		set.commit(transaction).await.unwrap();
		expect.remove(10);
		assert_eq!(set.stream(&storage).try_collect::<Vec<i32>>().await.unwrap(), expect);
	}
}
