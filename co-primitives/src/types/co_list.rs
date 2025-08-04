use super::lazy_transaction::Transactionable;
use crate::{
	library::lsm_tree_map::Root, BlockStorage, LazyTransaction, LsmTreeMap, OptionLink, StorageError, Streamable,
};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::{future::Either, stream::BoxStream, Stream, StreamExt, TryStreamExt};
use num_rational::Ratio;
use serde::{
	de::{DeserializeOwned, Error},
	Deserialize, Serialize,
};
use serde_bytes::Bytes;
use std::{future::Future, hash::Hash};
use unsigned_varint::{decode, encode};

/// CoList index - non continous.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoListIndex(Ratio<u64>);
impl CoListIndex {
	pub fn first() -> Self {
		Self(Ratio::new(1, 1))
	}

	pub fn prev(&self) -> Self {
		assert!(*self.0.numer() > 0);
		Self(Ratio::new(*self.0.numer(), self.0.denom() + 1))
	}

	pub fn next(&self) -> Self {
		Self(Ratio::from_integer(self.0.to_integer() + 1))
	}

	pub fn between(&self, other: &Self) -> Self {
		Self::mediant(self, other)
	}

	pub(crate) fn new_raw(numer: u64, denom: u64) -> Self {
		Self(Ratio::new_raw(numer, denom))
	}

	/// Find the mediant (best Farey approximation)
	///
	/// Note: This assumes x and y are in direct sequence.
	///
	/// # Formula
	///
	/// $`\text{mediant} = \frac{p_1 + p_2}{q_1 + q_2}`$
	fn mediant(x: &Self, y: &Self) -> Self {
		Self::new_raw(x.0.numer() + y.0.numer(), x.0.denom() + y.0.denom())
	}
}
impl Default for CoListIndex {
	fn default() -> Self {
		Self::first()
	}
}
impl Serialize for CoListIndex {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let value = self.0.into_raw();
		let mut buf = (encode::u64_buffer(), encode::u64_buffer());
		let enc = (Bytes::new(encode::u64(value.0, &mut buf.0)), Bytes::new(encode::u64(value.1, &mut buf.1)));
		enc.serialize(serializer)
	}
}
impl<'de> Deserialize<'de> for CoListIndex {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let bytes: (&Bytes, &Bytes) = Deserialize::deserialize(deserializer)?;
		let dec = (decode::u64(&bytes.0).map_err(D::Error::custom)?, decode::u64(&bytes.1).map_err(D::Error::custom)?);
		Ok(Self(Ratio::new(dec.0 .0, dec.1 .0)))
	}
}

/// CoList stored values in a sequence.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CoList<V>(OptionLink<Root<CoListIndex, V>>)
where
	V: Clone + Send + Sync + 'static;
impl<V> CoList<V>
where
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// Create collection from iterator.
	pub async fn from_iter<S>(storage: &S, iter: impl IntoIterator<Item = V>) -> Result<Self, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = Self::default().open(storage).await?;
		for value in iter.into_iter() {
			transaction.push(value).await?;
		}
		Ok(transaction.store().await?)
	}

	/// Whether this collection is empty.
	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	pub async fn get<S>(&self, storage: &S, key: &CoListIndex) -> Result<Option<V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.open(storage).await?.get(key).await
	}

	pub async fn contains<S>(&self, storage: &S, key: &CoListIndex) -> Result<bool, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		self.open(storage).await?.contains(key).await
	}

	pub fn stream<S>(&self, storage: &S) -> impl Stream<Item = Result<(CoListIndex, V), StorageError>> + '_
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

	/// Convenience method to load all or `limit` entries into memory.
	pub async fn vec<S>(&self, storage: &S, limit: Option<usize>) -> Result<Vec<V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let transaction = self.open(storage).await?;
		let stream = transaction.stream().map_ok(|(_index, value)| value);
		let stream = if let Some(limit) = limit { Either::Left(stream.take(limit)) } else { Either::Right(stream) };
		stream.try_collect().await
	}

	pub async fn insert<S>(&mut self, storage: &S, index: CoListIndex, value: V) -> Result<CoListIndex, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.insert(index, value).await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	pub async fn set<S>(&mut self, storage: &S, key: CoListIndex, value: V) -> Result<(), StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		transaction.set(key, value).await?;
		self.commit(transaction).await?;
		Ok(())
	}

	pub async fn push<S>(&mut self, storage: &S, value: V) -> Result<CoListIndex, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.push(value).await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	/// Pop last element.
	///
	/// See: [`CoListTransaction::pop`]
	pub async fn pop<S>(&mut self, storage: &S) -> Result<Option<(CoListIndex, V)>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.pop().await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	/// Pop first element.
	///
	/// See: [`CoListTransaction::pop_front`]
	pub async fn pop_front<S>(&mut self, storage: &S) -> Result<Option<(CoListIndex, V)>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.pop_front().await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	pub async fn remove<S>(&mut self, storage: &S, key: CoListIndex) -> Result<Option<V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let mut transaction = self.open(storage).await?;
		let result = transaction.remove(key).await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	/// Update (or insert default) value.
	pub async fn update_or_insert<S, F>(&mut self, storage: &S, key: CoListIndex, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(&mut V) -> () + Send,
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
		key: CoListIndex,
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
	pub async fn update<S, F>(&mut self, storage: &S, key: CoListIndex, update: F) -> Result<(), StorageError>
	where
		F: FnOnce(&mut V) -> () + Send,
		S: BlockStorage + Clone + 'static,
	{
		self.with_transaction(storage, |mut transaction| async move {
			transaction.update(key, update).await?;
			Ok(transaction)
		})
		.await
	}

	/// Update (or insert default) value.
	pub async fn try_update_async<S, F, Fut>(
		&mut self,
		storage: &S,
		key: CoListIndex,
		update: F,
	) -> Result<(), StorageError>
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

	pub async fn open<S>(&self, storage: &S) -> Result<CoListTransaction<S, V>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		Ok(CoListTransaction {
			tree: match self.0.link() {
				Some(root) => LsmTreeMap::load(storage.clone(), root).await?,
				None => LsmTreeMap::new(storage.clone(), Default::default()),
			},
			max_key: None,
		})
	}

	pub async fn open_lazy<S>(&self, storage: &S) -> Result<LazyTransaction<S, Self>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		Ok(LazyTransaction::new(storage.clone(), self.clone()))
	}

	pub async fn commit<S>(&mut self, mut transaction: CoListTransaction<S, V>) -> Result<(), StorageError>
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
		F: FnOnce(CoListTransaction<S, V>) -> Fut + Send,
		Fut: Future<Output = Result<CoListTransaction<S, V>, StorageError>> + Send,
	{
		let transaction = self.open(storage).await?;
		let mut result = update(transaction).await?;
		self.0 = result.tree.store().await?;
		Ok(())
	}
}
impl<V> Default for CoList<V>
where
	V: Clone + Send + Sync + 'static,
{
	fn default() -> Self {
		Self(Default::default())
	}
}
#[async_trait]
impl<S, V> Transactionable<S> for CoList<V>
where
	S: BlockStorage + Clone + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	type Transaction = CoListTransaction<S, V>;

	async fn open(&self, storage: &S) -> Result<Self::Transaction, StorageError> {
		CoList::open(self, storage).await
	}
}
impl<S, V> Streamable<S> for CoList<V>
where
	S: BlockStorage + Clone + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	type Item = Result<(CoListIndex, V), StorageError>;
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
pub struct CoListTransaction<S, V>
where
	S: BlockStorage + Clone + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	/// The tree.
	tree: LsmTreeMap<S, CoListIndex, V>,

	/// Cache for known max key.
	max_key: Option<Option<CoListIndex>>,
}
impl<S, V> CoListTransaction<S, V>
where
	S: BlockStorage + Clone + 'static,
	V: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
	async fn max_key(&mut self) -> Result<Option<CoListIndex>, StorageError> {
		if self.max_key.is_none() {
			self.max_key = Some(self.tree.max_key().await?);
		}
		if let Some(last_max_key) = self.max_key {
			Ok(last_max_key)
		} else {
			Ok(None)
		}
	}

	pub async fn get(&self, key: &CoListIndex) -> Result<Option<V>, StorageError> {
		self.tree.get(key).await
	}

	pub async fn contains(&self, key: &CoListIndex) -> Result<bool, StorageError> {
		self.tree.contains_key(key).await
	}

	pub fn stream(&self) -> impl Stream<Item = Result<(CoListIndex, V), StorageError>> + '_ {
		self.tree.stream()
	}

	pub fn reverse_stream(&self) -> impl Stream<Item = Result<(CoListIndex, V), StorageError>> + '_ {
		self.tree.reverse_stream()
	}

	/// Insert value after index shifting all elements after to the right.
	pub async fn insert(&mut self, index: CoListIndex, value: V) -> Result<CoListIndex, StorageError> {
		// find index + 1 items
		let items: Vec<(CoListIndex, V)> = self.tree.stream_query(Some(index)).take(2).try_collect().await?;
		let result = match (items.get(0), items.get(1)) {
			(Some(_), None) => index.next(),
			(Some((first, _)), Some((second, _))) => first.between(second),
			_ => return Err(StorageError::InvalidArgument(anyhow!("Index not found: {:?}", index))),
		};

		// set
		self.tree.insert(result, value).await?;

		// result
		Ok(result)
	}

	/// Insert value at index shifting all elements at or after to the right.
	pub async fn insert_before(&mut self, index: CoListIndex, value: V) -> Result<CoListIndex, StorageError> {
		// find index - 1 items
		let items: Vec<(CoListIndex, V)> = self.tree.reverse_stream_query(Some(index)).take(2).try_collect().await?;
		let result = match (items.get(0), items.get(1)) {
			(Some(_), None) => index.prev(),
			(Some((first, _)), Some((second, _))) => first.between(second),
			_ => return Err(StorageError::InvalidArgument(anyhow!("Index not found: {:?}", index))),
		};

		// set
		self.tree.insert(result, value).await?;

		// result
		Ok(result)
	}

	// /// Insert value between index shifting all elements after to the right.
	// /// TODO: this is broken as we dont reduce the keys?
	// pub async fn insert_between(
	// 	&mut self,
	// 	index: (CoListIndex, CoListIndex),
	// 	value: V,
	// ) -> Result<CoListIndex, StorageError> {
	// 	// create index
	// 	//  if the provided indices are in a direct sequence this will always work on first try
	// 	//  if the provided indices are not in a direct sequence we need to check if the resulting key not exists yet
	// 	let mut result = index.0.between(&index.1);
	// 	while self.tree.get(&result).await?.is_some() {
	// 		result = index.0.between(&result);
	// 	}
	//
	// 	// set
	// 	self.tree.insert(result, value).await?;
	//
	// 	// result
	// 	Ok(result)
	// }

	/// Insert (set/replace) value to index.
	pub async fn set(&mut self, key: CoListIndex, value: V) -> Result<(), StorageError> {
		self.tree.insert(key, value).await
	}

	/// Push as last value.
	pub async fn push(&mut self, value: V) -> Result<CoListIndex, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		let next_key = self.max_key().await?.unwrap_or_default().next();
		self.tree.insert(next_key, value).await?;
		self.max_key = Some(Some(next_key));
		Ok(next_key)
	}

	/// Pop last element.
	pub async fn pop(&mut self) -> Result<Option<(CoListIndex, V)>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		if let Some(key) = self.max_key().await? {
			self.max_key = None;
			Ok(self.remove(key.clone()).await?.map(|value| (key, value)))
		} else {
			Ok(None)
		}
	}

	/// Pop first element.
	pub async fn pop_front(&mut self) -> Result<Option<(CoListIndex, V)>, StorageError>
	where
		S: BlockStorage + Clone + 'static,
	{
		if let Some(key) = self.tree.min_key().await? {
			Ok(self.remove(key.clone()).await?.map(|value| (key, value)))
		} else {
			Ok(None)
		}
	}

	/// Remove given element and return the value.
	pub async fn remove(&mut self, key: CoListIndex) -> Result<Option<V>, StorageError> {
		if let Some(value) = self.tree.get(&key).await? {
			self.tree.remove(key).await?;
			Ok(Some(value))
		} else {
			Ok(None)
		}
	}

	/// Update (or insert default) value.
	pub async fn update_or_insert<F>(&mut self, key: CoListIndex, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(&mut V) -> () + Send,
	{
		let mut item = self.get(&key).await?.unwrap_or_default();
		update(&mut item);
		self.insert(key, item).await?;
		Ok(())
	}

	/// Update (or insert default) value.
	pub async fn try_update_or_insert_async<F, Fut>(&mut self, key: CoListIndex, update: F) -> Result<(), StorageError>
	where
		V: Default,
		F: FnOnce(V) -> Fut + Send,
		Fut: Future<Output = Result<V, StorageError>> + Send,
	{
		let item = self.get(&key).await?.unwrap_or_default();
		let next_item = update(item).await?;
		self.set(key, next_item).await?;
		Ok(())
	}

	/// Update value, ignore if key not exists.
	pub async fn update<F>(&mut self, key: CoListIndex, update: F) -> Result<(), StorageError>
	where
		F: FnOnce(&mut V) -> () + Send,
	{
		if let Some(mut item) = self.get(&key).await? {
			update(&mut item);
			self.insert(key, item).await?;
		}
		Ok(())
	}

	/// Update value, ignore if key not exists.
	pub async fn try_update_async<F, Fut>(&mut self, key: CoListIndex, update: F) -> Result<(), StorageError>
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

	/// Store as new [`CoList`]
	pub async fn store(&mut self) -> Result<CoList<V>, StorageError> {
		let link = self.tree.store().await?;
		Ok(CoList(link))
	}
}

#[cfg(test)]
mod tests {
	use super::CoListIndex;
	use crate::{library::test::TestStorage, BlockSerializer, CoList};
	use futures::TryStreamExt;

	#[tokio::test]
	async fn test_push() {
		let storage = TestStorage::default();
		let mut list = CoList::default();
		let mut transaction = list.open(&storage).await.unwrap();
		transaction.push(1).await.unwrap();
		transaction.push(2).await.unwrap();
		transaction.push(3).await.unwrap();
		transaction.push(4).await.unwrap();
		list.commit(transaction).await.unwrap();
		assert_eq!(
			list.stream(&storage)
				.map_ok(|(_key, value)| value)
				.try_collect::<Vec<_>>()
				.await
				.unwrap(),
			vec![1, 2, 3, 4]
		);
	}

	#[tokio::test]
	async fn test_pop() {
		let storage = TestStorage::default();
		let mut list = CoList::default();
		let mut transaction = list.open(&storage).await.unwrap();
		transaction.push(1).await.unwrap();
		transaction.push(2).await.unwrap();
		transaction.push(3).await.unwrap();
		transaction.push(4).await.unwrap();
		list.commit(transaction).await.unwrap();
		assert_eq!(list.pop(&storage).await.unwrap().unwrap().1, 4);
		assert_eq!(
			list.stream(&storage)
				.map_ok(|(_key, value)| value)
				.try_collect::<Vec<_>>()
				.await
				.unwrap(),
			vec![1, 2, 3]
		);
	}

	#[tokio::test]
	async fn test_insert() {
		let storage = TestStorage::default();
		let mut list = CoList::default();
		let mut transaction = list.open(&storage).await.unwrap();
		transaction.push(1).await.unwrap();
		let two = transaction.push(2).await.unwrap();
		transaction.push(3).await.unwrap();
		transaction.push(4).await.unwrap();
		transaction.insert(two, 22).await.unwrap();
		list.commit(transaction).await.unwrap();
		assert_eq!(
			list.stream(&storage)
				.map_ok(|(_key, value)| value)
				.try_collect::<Vec<_>>()
				.await
				.unwrap(),
			vec![1, 2, 22, 3, 4]
		);
	}

	#[tokio::test]
	async fn test_insert_last() {
		let storage = TestStorage::default();
		let mut list = CoList::default();
		let mut transaction = list.open(&storage).await.unwrap();
		transaction.push(1).await.unwrap();
		transaction.push(2).await.unwrap();
		transaction.push(3).await.unwrap();
		let four = transaction.push(4).await.unwrap();
		transaction.insert(four, 44).await.unwrap();
		list.commit(transaction).await.unwrap();
		assert_eq!(
			list.stream(&storage)
				.map_ok(|(_key, value)| value)
				.try_collect::<Vec<_>>()
				.await
				.unwrap(),
			vec![1, 2, 3, 4, 44]
		);
	}

	#[tokio::test]
	async fn test_insert_before() {
		let storage = TestStorage::default();
		let mut list = CoList::default();
		let mut transaction = list.open(&storage).await.unwrap();
		transaction.push(1).await.unwrap();
		let two = transaction.push(2).await.unwrap();
		transaction.push(3).await.unwrap();
		transaction.push(4).await.unwrap();
		transaction.insert_before(two, 22).await.unwrap();
		list.commit(transaction).await.unwrap();
		assert_eq!(
			list.stream(&storage)
				.map_ok(|(_key, value)| value)
				.try_collect::<Vec<_>>()
				.await
				.unwrap(),
			vec![1, 22, 2, 3, 4]
		);
	}

	#[tokio::test]
	async fn test_insert_before_first() {
		let storage = TestStorage::default();
		let mut list = CoList::default();
		let mut transaction = list.open(&storage).await.unwrap();
		let one = transaction.push(1).await.unwrap();
		transaction.push(2).await.unwrap();
		transaction.push(3).await.unwrap();
		transaction.push(4).await.unwrap();
		transaction.insert_before(one, 11).await.unwrap();
		list.commit(transaction).await.unwrap();
		assert_eq!(
			list.stream(&storage)
				.map_ok(|(_key, value)| value)
				.try_collect::<Vec<_>>()
				.await
				.unwrap(),
			vec![11, 1, 2, 3, 4]
		);
	}

	#[test]
	fn test_index_serialize() {
		let block = BlockSerializer::default().serialize(&CoListIndex::first()).unwrap();
		let index: CoListIndex = BlockSerializer::default().deserialize(&block).unwrap();
		assert_eq!(block.data().len(), 5);
		assert_eq!(index, CoListIndex::first());
	}

	#[test]
	fn test_index_next() {
		assert_eq!(CoListIndex::first().next(), CoListIndex::new_raw(2, 1));
		assert_eq!(CoListIndex::new_raw(5, 2).next(), CoListIndex::new_raw(3, 1));
		assert_eq!(CoListIndex::new_raw(7, 3).next(), CoListIndex::new_raw(3, 1));
	}

	#[test]
	fn test_index_prev() {
		assert_eq!(CoListIndex::default().prev(), CoListIndex::new_raw(1, 2));
		assert_eq!(CoListIndex::first().prev(), CoListIndex::new_raw(1, 2));
		assert_eq!(CoListIndex::new_raw(1, 2).prev(), CoListIndex::new_raw(1, 3));
		assert_eq!(CoListIndex::new_raw(1, 3).prev(), CoListIndex::new_raw(1, 4));
		assert_eq!(CoListIndex::new_raw(5, 2).prev(), CoListIndex::new_raw(5, 3));
	}

	#[test]
	fn test_index_between() {
		let low = CoListIndex::new_raw(2, 1);
		let high = CoListIndex::new_raw(3, 1);
		let med = low.between(&high);
		assert_eq!(med, CoListIndex::new_raw(5, 2));
		let med = low.between(&med);
		assert_eq!(med, CoListIndex::new_raw(7, 3));
	}

	#[test]
	fn test_index_between_repeat() {
		let low = CoListIndex::new_raw(2, 1);
		let high = low.next();
		let mut med = high;
		for _ in [0; 10000] {
			let next_med = low.between(&med);
			assert_ne!(next_med, low);
			assert_ne!(next_med, high);
			assert_ne!(next_med, med);
			assert!(next_med > low);
			assert!(next_med < med);
			assert!(next_med < high);
			med = next_med;
		}
	}
}
