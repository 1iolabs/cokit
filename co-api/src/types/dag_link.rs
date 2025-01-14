use crate::{library::node_reader::node_reader, Context, NodeReaderError, Storage};
use co_primitives::{Node, NodeBuilder, NodeContainer, OptionLink};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	cmp::Ord,
	collections::{BTreeMap, BTreeSet},
};

/// Simple trait for creating a DagLink type object
pub trait DagCollection: Sized {
	type Item: Clone + Serialize + DeserializeOwned + 'static;
	type Collection: Default + Clone + IntoIterator<Item = Self::Item> + FromIterator<Self::Item>;

	fn link(&self) -> OptionLink<Self::Collection>;
	fn set_link(&mut self, link: OptionLink<Self::Collection>);

	/// Replace contents with collection.
	fn set_collection(&mut self, storage: &mut dyn Storage, items: Self::Collection) {
		self.set_link(Self::to_link(storage, items))
	}

	/// Materialize into the collection.
	fn collection(&self, storage: &dyn Storage) -> Self::Collection {
		self.from_link(storage).expect("Valid serialized data")
	}

	fn update<F: FnOnce(&mut dyn Context, &mut Self::Collection) -> R, R>(
		&mut self,
		context: &mut dyn Context,
		f: F,
	) -> R {
		let mut collection = self.collection(context.storage());
		let result = f(context, &mut collection);
		self.set_collection(context.storage_mut(), collection);
		result
	}

	fn try_update<F: FnOnce(&mut dyn Context, &mut Self::Collection) -> Result<R, anyhow::Error>, R>(
		&mut self,
		context: &mut dyn Context,
		f: F,
	) -> Result<R, anyhow::Error> {
		let mut collection = self.collection(context.storage());
		let result = f(context, &mut collection)?;
		self.set_collection(context.storage_mut(), collection);
		Ok(result)
	}

	fn update_owned<F: FnOnce(&mut dyn Context, Self::Collection) -> Self::Collection>(
		&mut self,
		context: &mut dyn Context,
		f: F,
	) {
		let mut collection = self.collection(context.storage());
		collection = f(context, collection);
		self.set_collection(context.storage_mut(), collection);
	}

	fn iter(&self, storage: &dyn Storage) -> impl Iterator<Item = Self::Item> {
		node_reader::<Self::Item>(storage, *self.link().cid()).map(|item| item.expect("Valid serialized data"))
	}

	fn try_iter(&self, storage: &dyn Storage) -> impl Iterator<Item = Result<Self::Item, NodeReaderError>> {
		node_reader::<Self::Item>(storage, *self.link().cid())
	}

	fn to_link(storage: &mut dyn Storage, items: impl IntoIterator<Item = Self::Item>) -> OptionLink<Self::Collection> {
		let mut node_builder = NodeBuilder::<Self::Item>::default();
		for item in items {
			node_builder.push(item).unwrap();
		}
		let blocks = node_builder.into_blocks().unwrap();
		let mut result = OptionLink::none();
		for block in blocks {
			let cid = storage.set(block);
			if result.is_none() {
				result.set(Some(cid));
			}
		}
		result
	}

	fn from_link(&self, storage: &dyn Storage) -> Result<Self::Collection, NodeReaderError> {
		self.try_iter(storage).collect()
	}
}

/// A wrapper type for DagLink types that use vectors
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DagVec<V>(OptionLink<Vec<V>>);
impl<V> DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	pub fn create(storage: &mut dyn Storage, items: impl IntoIterator<Item = <Self as DagCollection>::Item>) -> Self {
		Self(Self::to_link(storage, items))
	}

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	///
	/// TODO: Do not load whole collection into memory.
	pub fn update_one<R>(
		&mut self,
		context: &mut dyn Context,
		predicate: impl Fn(&mut dyn Context, &V) -> bool,
		update: impl FnOnce(&mut dyn Context, &mut V) -> R,
	) -> Option<R> {
		let mut collection = self.collection(context.storage());
		for item in collection.iter_mut() {
			if predicate(context, item) {
				let result = update(context, item);
				self.set_collection(context.storage_mut(), collection);
				return Some(result);
			}
		}
		None
	}
}
impl<V> Clone for DagVec<V> {
	fn clone(&self) -> Self {
		Self(self.0)
	}
}
impl<V> Default for DagVec<V> {
	fn default() -> Self {
		Self(OptionLink::none())
	}
}
impl<V> DagCollection for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = V;
	type Collection = Vec<Self::Item>;

	fn link(&self) -> OptionLink<Self::Collection> {
		self.0
	}

	fn set_link(&mut self, link: OptionLink<Self::Collection>) {
		self.0 = link;
	}
}
impl<V> NodeContainer<V> for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	fn node_container_link(&self) -> OptionLink<Node<V>> {
		OptionLink::new(*self.0.cid())
	}
}

/// A wrapper for DagLink types that use the BTreeSet type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DagSet<V: Ord>(OptionLink<BTreeSet<V>>);
impl<V> DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	pub fn create(storage: &mut dyn Storage, items: impl IntoIterator<Item = <Self as DagCollection>::Item>) -> Self {
		Self(Self::to_link(storage, items))
	}

	/// Returns `true` if the set contains no elements.
	pub fn is_empty(&self) -> bool {
		self.0.is_none()
	}

	/// Adds a value to the set.
	///
	/// TODO: (perf): Do not load whole set into memory
	pub fn insert(&mut self, storage: &mut dyn Storage, value: V) -> bool {
		let mut set = self.collection(storage);
		if set.insert(value) {
			self.set_collection(storage, set);
			true
		} else {
			false
		}
	}

	/// Remove a value from the set.
	///
	/// TODO: (perf): Do not load whole set into memory
	pub fn remove(&mut self, storage: &mut dyn Storage, value: &V) -> bool {
		let mut set = self.collection(storage);
		if set.remove(value) {
			self.set_collection(storage, set);
			true
		} else {
			false
		}
	}

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	///
	/// TODO: Do not load whole collection into memory.
	pub fn update_one<R>(
		&mut self,
		context: &mut dyn Context,
		predicate: impl Fn(&mut dyn Context, &V) -> bool,
		update: impl FnOnce(&mut dyn Context, &mut V) -> R,
	) -> Option<R> {
		let mut collection = self.collection(context.storage());
		if let Some(mut item) = collection.iter().find(|item| predicate(context, item)).cloned() {
			if collection.remove(&item) {
				// update
				let result = update(context, &mut item);

				// insert
				collection.insert(item);
				self.set_collection(context.storage_mut(), collection);
				return Some(result);
			}
		}
		None
	}

	/// Update one element that matches the predicate.
	/// Returns Some result if a item has been updated and None otherwise.
	/// If the update fails it will be not applied.
	///
	/// TODO: Do not load whole collection into memory.
	pub fn try_update_one<R>(
		&mut self,
		context: &mut dyn Context,
		predicate: impl Fn(&mut dyn Context, &V) -> bool,
		update: impl FnOnce(&mut dyn Context, &mut V) -> Result<R, anyhow::Error>,
	) -> Result<Option<R>, anyhow::Error> {
		let mut collection = self.collection(context.storage());
		if let Some(mut item) = collection.iter().find(|item| predicate(context, item)).cloned() {
			if collection.remove(&item) {
				// update
				let result = update(context, &mut item)?;

				// insert
				collection.insert(item);
				self.set_collection(context.storage_mut(), collection);
				return Ok(Some(result));
			}
		}
		Ok(None)
	}
}
impl<V> DagCollection for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = V;
	type Collection = BTreeSet<Self::Item>;

	fn link(&self) -> OptionLink<Self::Collection> {
		self.0
	}

	fn set_link(&mut self, link: OptionLink<Self::Collection>) {
		self.0 = link;
	}
}
impl<V> Default for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn default() -> Self {
		Self(OptionLink::none())
	}
}
impl<V> NodeContainer<V> for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn node_container_link(&self) -> OptionLink<Node<V>> {
		OptionLink::new(*self.0.cid())
	}
}

/// A wrapper for DagLink types that use the BTreeMap type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DagMap<K, V>(OptionLink<BTreeMap<K, V>>)
where
	K: Ord + Clone + Serialize,
	V: Clone + Serialize;
impl<K, V> DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	pub fn create(storage: &mut dyn Storage, items: impl IntoIterator<Item = <Self as DagCollection>::Item>) -> Self {
		Self(Self::to_link(storage, items))
	}

	/// Returns a value corresponding to the key.
	pub fn get(&mut self, context: &mut dyn Context, key: &K) -> Option<V> {
		self.iter(context.storage())
			.find(|(item_key, _item_value)| key == item_key)
			.map(|(_item_key, item_value)| item_value)
	}

	/// Inserts a key-value pair into the map.
	///
	/// TODO: Do not load whole collection into memory.
	pub fn insert(&mut self, context: &mut dyn Context, key: K, value: V) -> Option<V> {
		self.update(context, |_, v| v.insert(key, value))
	}

	/// Removes a key from the map, returning the value at the key if the key
	/// was previously in the map.
	///
	/// TODO: Do not load whole collection into memory.
	pub fn remove(&mut self, context: &mut dyn Context, key: &K) -> Option<V> {
		self.update(context, |_, v| v.remove(key))
	}

	/// Update element with given key.
	/// Returns Some result if a item has been updated and None otherwise.
	pub fn update_key<R>(
		&mut self,
		context: &mut dyn Context,
		key: &K,
		update: impl FnOnce(&mut dyn Context, &K, &mut V) -> R,
	) -> Option<R> {
		self.update(context, move |context, map| {
			if let Some(mut item) = map.remove(key) {
				let result = update(context, key, &mut item);
				map.insert(key.clone(), item);
				return Some(result);
			}
			None
		})
	}

	/// Update element with given key.
	/// Returns Some result if the key was found and modified None otherwise.
	pub fn try_update_key<R>(
		&mut self,
		context: &mut dyn Context,
		key: &K,
		update: impl FnOnce(&mut dyn Context, &K, &mut V) -> Result<R, anyhow::Error>,
	) -> Result<Option<R>, anyhow::Error> {
		self.try_update(context, move |context, map| {
			if let Some(mut item) = map.remove(key) {
				let result = update(context, key, &mut item)?;
				map.insert(key.clone(), item);
				return Ok(Some(result));
			}
			Ok(None)
		})
	}
}
impl<K, V> DagCollection for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = (K, V);
	type Collection = BTreeMap<K, V>;

	fn link(&self) -> OptionLink<Self::Collection> {
		self.0
	}

	fn set_link(&mut self, link: OptionLink<Self::Collection>) {
		self.0 = link;
	}
}
impl<K, V> Default for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn default() -> Self {
		Self(Default::default())
	}
}
impl<K, V> NodeContainer<(K, V)> for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn node_container_link(&self) -> OptionLink<Node<(K, V)>> {
		OptionLink::new(*self.0.cid())
	}
}

#[cfg(test)]
mod test {
	use super::DagSet;
	use crate::{
		types::dag_link::{DagCollection, DagVec},
		DagMap,
	};
	use cid::Cid;
	use co_primitives::{Block, DefaultParams};
	use co_storage::{MemoryStorage, Storage};
	use std::collections::{BTreeMap, BTreeSet};

	struct TestStorage {
		mem_storage: MemoryStorage,
	}
	impl crate::Storage for TestStorage {
		fn get(&self, cid: &Cid) -> Block<DefaultParams> {
			self.mem_storage.get(cid).expect("get")
		}

		fn set(&mut self, block: Block<DefaultParams>) -> Cid {
			self.mem_storage.set(block).expect("set")
		}
	}
	#[test]
	fn test_vec() {
		let mut s = TestStorage { mem_storage: MemoryStorage::new() };
		let original_vec = vec![
			"test".to_owned(),
			"testy".to_owned(),
			"zesty".to_owned(),
			"some".to_owned(),
			"more".to_owned(),
			"strings".to_owned(),
			"to".to_owned(),
			"test".to_owned(),
			"memory".to_owned(),
			"usage".to_owned(),
		];
		let dag_vec: DagVec<String> = DagVec::create(&mut s, original_vec.clone());
		let restored_vec = dag_vec.collection(&s);
		let json = serde_json::to_string_pretty(&dag_vec).unwrap();
		println!("Serialized: {json}");
		println!("Original vector: {:?}", original_vec);
		println!(
			"Sizes:\n\tPure data: {:?}\n\tLink): {:?}",
			std::mem::size_of_val(&*original_vec), // should grow with vector size
			std::mem::size_of_val(&dag_vec)        // should stay at 96 bit
		);
		assert_eq!(original_vec, restored_vec)
	}

	#[test]
	fn test_set() {
		let mut original_set: BTreeSet<String> = BTreeSet::new();

		original_set.insert("test".into());
		original_set.insert("testy".into());
		original_set.insert("test".into());
		original_set.insert("zesty".into());

		let mut s = TestStorage { mem_storage: MemoryStorage::new() };
		let dag_set = DagSet::create(&mut s, original_set.clone());
		let restored_set = dag_set.collection(&s);
		let json = serde_json::to_string_pretty(&dag_set).unwrap();
		println!("Serialized: {json}");
		assert_eq!(original_set, restored_set);
	}

	#[test]
	fn test_map() {
		let mut original_map: BTreeMap<String, String> = BTreeMap::new();

		original_map.insert("test".into(), "test".into());
		original_map.insert("testy".into(), "testy".into());
		original_map.insert("test".into(), "test".into());
		original_map.insert("zesty".into(), "zesty".into());

		let mut s = TestStorage { mem_storage: MemoryStorage::new() };
		let dag_map = DagMap::create(&mut s, original_map.clone());
		let restored_map = dag_map.collection(&s);
		assert_eq!(original_map, restored_map);
	}
}
