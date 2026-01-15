use super::lazy_transaction::Transactionable;
use crate::{
	library::lsm_tree_map::Root, BlockStorage, LazyTransaction, LsmTreeMap, OptionLink, StorageError, Streamable,
};
use async_trait::async_trait;
use cid::Cid;
use futures::{pin_mut, stream::BoxStream, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	future::{ready, Future},
	hash::Hash,
};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CoMap<K, V>(OptionLink<Root<K, V>>)
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static;
impl<K, V> CoMap<K, V>
where
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Create collection from iterator.
	pub async fn from_iter<S>(storage: &S, iter: impl IntoIterator<Item = (K, V)>) -> Result<Self, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = Self::default().open(storage).await?;
		for (key, value) in iter.into_iter() {
			transaction.insert(key, value).await?;
		}
		transaction.store().await
	}

	/// Whether this collection is empty.
	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	pub async fn get<S>(&self, storage: &S, key: &K) -> Result<Option<V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.open(storage).await?.get(key).await
	}

	pub async fn contains<S>(&self, storage: &S, key: &K) -> Result<bool, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.open(storage).await?.contains_key(key).await
	}

	pub fn stream<S>(&self, storage: &S) -> impl Stream<Item = Result<(K, V), StorageError>> + '_
	where
		S: BlockStorage + Clone + 'static,
	{
		let storage = storage.clone();
		async_stream::try_stream! {
			let tree = self.open(&storage).await?;
			let stream = tree.stream();
			for await item in stream {
				yield item?;
			}
		}
	}

	pub async fn insert<S>(&mut self, storage: &S, key: K, value: V) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.with_transaction(storage, |mut transaction| async move {
			transaction.insert(key, value).await?;
			Ok(transaction)
		})
		.await
	}

	pub async fn remove<S>(&mut self, storage: &S, key: K) -> Result<Option<V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.remove(key).await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	/// Update (or insert default) value.
	pub async fn update_or_insert<S, F>(&mut self, storage: &S, key: K, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(&mut V) + Send,
		S: BlockStorage + Clone + 'static,
	{
		self.with_transaction(storage, |mut transaction| async move {
			transaction.update_or_insert(key, update).await?;
			Ok(transaction)
		})
		.await
	}

	/// Update (or insert default) value.
	pub async fn try_update_or_insert_async<S, F, Fut>(
		&mut self,
		storage: &S,
		key: K,
		update: F,
	) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(V) -> Fut + Send,
		Fut: Future<Output = Result<V, StorageError>> + Send,
		S: BlockStorage + Clone + 'static,
	{
		self.with_transaction(storage, |mut transaction| async move {
			transaction.try_update_or_insert_async(key, update).await?;
			Ok(transaction)
		})
		.await
	}

	/// Update value ignore if key not exists.
	pub async fn update<S, F>(&mut self, storage: &S, key: K, update: F) -> Result<(), StorageError>
	where
		F: FnOnce(&mut V) + Send,
		S: BlockStorage + Clone + 'static,
	{
		self.with_transaction(storage, |mut transaction| async move {
			transaction.update(key, update).await?;
			Ok(transaction)
		})
		.await
	}

	/// Update (or insert default) value.
	pub async fn try_update_async<S, F, Fut>(&mut self, storage: &S, key: K, update: F) -> Result<(), StorageError>
	where
		F: FnOnce(V) -> Fut + Send,
		Fut: Future<Output = Result<V, StorageError>> + Send,
		S: BlockStorage + Clone + 'static,
	{
		self.with_transaction(storage, |mut transaction| async move {
			transaction.try_update_async(key, update).await?;
			Ok(transaction)
		})
		.await
	}

	pub async fn open_mut<'m, S>(&'m mut self, storage: &S) -> Result<CoMapMutTransaction<'m, S, K, V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		Ok(CoMapMutTransaction { transaction: self.open(storage).await?, container: self })
	}

	pub async fn open<S>(&self, storage: &S) -> Result<CoMapTransaction<S, K, V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		Ok(CoMapTransaction {
			tree: match self.0.link() {
				Some(root) => LsmTreeMap::load(storage.clone(), root).await?,
				None => LsmTreeMap::new(storage.clone(), Default::default()),
			},
		})
	}

	pub async fn open_lazy<S>(&self, storage: &S) -> Result<LazyTransaction<S, Self>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		Ok(LazyTransaction::new(storage.clone(), self.clone()))
	}

	/// Commit transaction to this map.
	pub async fn commit<S>(&mut self, mut transaction: CoMapTransaction<S, K, V>) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.0 = transaction.tree.store().await?;
		Ok(())
	}

	/// Open transaction, apply `update` and store it.
	pub async fn with_transaction<S, F, Fut>(&mut self, storage: &S, update: F) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
		F: FnOnce(CoMapTransaction<S, K, V>) -> Fut + Send,
		Fut: Future<Output = Result<CoMapTransaction<S, K, V>, StorageError>> + Send,
	{
		let transaction = self.open(storage).await?;
		let mut result = update(transaction).await?;
		self.0 = result.tree.store().await?;
		Ok(())
	}
}
impl<K, V> Default for CoMap<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self(Default::default())
	}
}
impl<K, V> From<Option<Cid>> for CoMap<K, V>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	fn from(cid: Option<Cid>) -> Self {
		Self(cid.into())
	}
}
impl<K, V> From<&CoMap<K, V>> for Option<Cid>
where
	K: Hash + Ord + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	fn from(value: &CoMap<K, V>) -> Self {
		*value.0.cid()
	}
}
#[async_trait]
impl<S, K, V> Transactionable<S> for CoMap<K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	type Transaction = CoMapTransaction<S, K, V>;

	async fn open(&self, storage: &S) -> Result<Self::Transaction, StorageError> {
		CoMap::open(self, storage).await
	}
}
impl<S, K, V> Streamable<S> for CoMap<K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	type Item = Result<(K, V), StorageError>;
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

pub struct CoMapMutTransaction<'m, S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	container: &'m mut CoMap<K, V>,
	transaction: CoMapTransaction<S, K, V>,
}
impl<'m, S, K, V> CoMapMutTransaction<'m, S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	pub async fn commit(mut self) -> Result<(), StorageError> {
		self.container.0 = self.transaction.tree.store().await?;
		Ok(())
	}
}
impl<'m, S, K, V> CoMapMutTransaction<'m, S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	pub async fn get(&self, key: &K) -> Result<Option<V>, StorageError> {
		self.transaction.get(key).await
	}

	pub async fn contains_key(&self, key: &K) -> Result<bool, StorageError> {
		self.transaction.contains_key(key).await
	}

	pub fn stream(&self) -> impl Stream<Item = Result<(K, V), StorageError>> + '_ {
		self.transaction.stream()
	}

	pub async fn insert(&mut self, key: K, value: V) -> Result<(), StorageError> {
		self.transaction.insert(key, value).await
	}

	pub async fn remove(&mut self, key: K) -> Result<Option<V>, StorageError> {
		self.transaction.remove(key).await
	}

	/// Update (or insert default) value.
	pub async fn try_update_or_insert_async<F, Fut>(&mut self, key: K, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(V) -> Fut + Send,
		Fut: Future<Output = Result<V, StorageError>> + Send,
	{
		self.transaction.try_update_or_insert_async(key, update).await
	}

	/// Store as new CoMap
	pub async fn store(&mut self) -> Result<CoMap<K, V>, StorageError> {
		self.transaction.store().await
	}
}

#[derive(Clone)]
pub struct CoMapTransaction<S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	tree: LsmTreeMap<S, K, V>,
}
impl<S, K, V> CoMapTransaction<S, K, V>
where
	S: BlockStorage + Clone + 'static,
	K: Hash + Ord + Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	pub async fn get(&self, key: &K) -> Result<Option<V>, StorageError> {
		self.tree.get(key).await
	}

	pub async fn contains_key(&self, key: &K) -> Result<bool, StorageError> {
		self.tree.contains_key(key).await
	}

	pub fn stream(&self) -> impl Stream<Item = Result<(K, V), StorageError>> + use<S, K, V> {
		self.tree.stream()
	}

	pub fn stream_filter<F: FnMut(&V) -> bool>(
		&self,
		mut predicate: F,
	) -> impl Stream<Item = Result<K, StorageError>> + use<S, K, V, F> {
		self.stream()
			.try_filter_map(move |(key, value)| ready(Ok(if predicate(&value) { Some(key) } else { None })))
	}

	pub async fn insert(&mut self, key: K, value: V) -> Result<(), StorageError> {
		self.tree.insert(key, value).await
	}

	pub async fn remove(&mut self, key: K) -> Result<Option<V>, StorageError> {
		if let Some(value) = self.tree.get(&key).await? {
			self.tree.remove(key).await?;
			Ok(Some(value))
		} else {
			Ok(None)
		}
	}

	/// Update (or insert default) value.
	pub async fn update_or_insert<F>(&mut self, key: K, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(&mut V) + Send,
	{
		let mut item = self.get(&key).await?.unwrap_or_default();
		update(&mut item);
		self.insert(key, item).await?;
		Ok(())
	}

	/// Update (or insert default) value.
	pub async fn try_update_or_insert_async<F, Fut>(&mut self, key: K, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(V) -> Fut + Send,
		Fut: Future<Output = Result<V, StorageError>> + Send,
	{
		let item = self.get(&key).await?.unwrap_or_default();
		let next_item = update(item).await?;
		self.insert(key, next_item).await?;
		Ok(())
	}

	/// Update value, ignore if key not exists.
	pub async fn update<F>(&mut self, key: K, update: F) -> Result<(), StorageError>
	where
		F: FnOnce(&mut V) + Send,
	{
		if let Some(mut item) = self.get(&key).await? {
			update(&mut item);
			self.insert(key, item).await?;
		}
		Ok(())
	}

	/// Update value, ignore if key not exists.
	pub async fn try_update_async<F, Fut>(&mut self, key: K, update: F) -> Result<(), StorageError>
	where
		F: FnOnce(V) -> Fut + Send,
		Fut: Future<Output = Result<V, StorageError>> + Send,
	{
		if let Some(item) = self.get(&key).await? {
			let next_item = update(item).await?;
			self.insert(key, next_item).await?;
		}
		Ok(())
	}

	pub async fn update_stream(
		&mut self,
		keys_to_update: impl Stream<Item = Result<K, StorageError>>,
		mut update: impl FnMut(&K, &mut V) + Send,
	) -> Result<(), StorageError> {
		pin_mut!(keys_to_update);
		while let Some(key) = keys_to_update.try_next().await? {
			if let Some(mut value) = self.get(&key).await? {
				(update)(&key, &mut value);
				self.insert(key, value).await?;
			}
		}
		Ok(())
	}

	pub async fn remove_stream(
		&mut self,
		keys_to_remove: impl Stream<Item = Result<K, StorageError>>,
	) -> Result<(), StorageError> {
		pin_mut!(keys_to_remove);
		while let Some(key) = keys_to_remove.try_next().await? {
			self.remove(key).await?;
		}
		Ok(())
	}

	/// Store as new CoMap
	pub async fn store(&mut self) -> Result<CoMap<K, V>, StorageError> {
		let link = self.tree.store().await?;
		Ok(CoMap(link))
	}
}

#[cfg(test)]
mod tests {
	use crate::{library::test::TestStorage, CoMap};
	use futures::TryStreamExt;
	use std::time::SystemTime;

	#[tokio::test]
	async fn smoke() {
		let storage = TestStorage::default();
		let mut map = CoMap::<i32, i32>::default();
		let mut transaction = map.open(&storage).await.unwrap();
		transaction.insert(1, 1).await.unwrap();
		transaction.insert(2, 2).await.unwrap();
		map.commit(transaction).await.unwrap();
		assert_eq!(map.stream(&storage).try_collect::<Vec<(i32, i32)>>().await.unwrap(), vec![(1, 1), (2, 2)]);
	}

	const BENCHMARK_REPEATS: i32 = 1000;
	#[tokio::test]
	async fn benchmark_transactional() {
		let ts = SystemTime::now();
		let storage = TestStorage::default();
		let mut map = CoMap::<i32, i32>::default();
		let mut transaction = map.open(&storage).await.unwrap();
		for i in 0..BENCHMARK_REPEATS {
			transaction.insert(i, i).await.unwrap();
		}
		map.commit(transaction).await.unwrap();
		println!(
			"{} insert transactions done in: {:?} seconds",
			BENCHMARK_REPEATS,
			SystemTime::now().duration_since(ts).unwrap().as_secs_f32()
		);
	}

	#[tokio::test]
	async fn benchmark_pure() {
		let ts = SystemTime::now();
		let storage = TestStorage::default();
		let mut map = CoMap::<i32, i32>::default();
		for i in 0..BENCHMARK_REPEATS {
			map.insert(&storage, i, i).await.unwrap();
		}
		println!(
			"{} pure inserts done in: {:?} seconds",
			BENCHMARK_REPEATS,
			SystemTime::now().duration_since(ts).unwrap().as_secs_f32()
		);
	}
}
