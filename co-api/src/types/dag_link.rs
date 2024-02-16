use crate::{library::node_reader::node_reader, NodeReaderError, Storage};
use co_primitives::{Link, Linkable, NodeBuilder, NodeContainer};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	cmp::Ord,
	collections::{BTreeMap, BTreeSet},
};

/// Simple trait for creating a DagLink type object
pub trait DagCollection: Sized {
	type Item: Clone + Serialize + DeserializeOwned + 'static;
	type Collection: Clone + IntoIterator<Item = Self::Item> + FromIterator<Self::Item>;

	fn link(&self) -> Option<Link<Self::Collection>>;
	fn set_link(&mut self, link: Option<Link<Self::Collection>>);

	fn set(&mut self, storage: &mut dyn Storage, items: Self::Collection) {
		self.set_link(Self::to_link(storage, items.into_iter()))
	}

	fn get(&self, storage: &dyn Storage) -> Self::Collection {
		self.from_link(storage).expect("Valid serialized data")
	}

	fn iter(&self, storage: &dyn Storage) -> impl Iterator<Item = Self::Item> {
		node_reader::<Self::Item>(storage, self.link().map(|link| link.cid().clone()))
			.map(|item| item.expect("Valid serialized data"))
	}

	fn try_iter(&self, storage: &dyn Storage) -> impl Iterator<Item = Result<Self::Item, NodeReaderError>> {
		node_reader::<Self::Item>(storage, self.link().map(|link| link.cid().clone()))
	}

	fn to_link(
		storage: &mut dyn Storage,
		items: impl IntoIterator<Item = Self::Item>,
	) -> Option<Link<Self::Collection>> {
		let mut node_builder = NodeBuilder::<Self::Item>::default();
		for item in items {
			node_builder.push(item).unwrap();
		}
		let blocks = node_builder.into_blocks().unwrap();
		let mut result = None;
		for block in blocks {
			let cid = storage.set(block);
			if result.is_none() {
				result = Some(Link::new(cid));
			}
		}
		result
	}

	fn from_link(&self, storage: &dyn Storage) -> Result<Self::Collection, NodeReaderError> {
		self.try_iter(storage).collect()
	}
}

/// A wrapper type for DagLink types that use vectors
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DagVec<V>(Option<Link<Vec<V>>>);
impl<V> DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	pub fn create(storage: &mut dyn Storage, items: impl IntoIterator<Item = <Self as DagCollection>::Item>) -> Self {
		Self(Self::to_link(storage, items))
	}
}
impl<V> Clone for DagVec<V> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
impl<V> Default for DagVec<V> {
	fn default() -> Self {
		Self(None)
	}
}
impl<V> DagCollection for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = V;
	type Collection = Vec<Self::Item>;

	fn link(&self) -> Option<Link<Self::Collection>> {
		self.0.clone()
	}

	fn set_link(&mut self, link: Option<Link<Self::Collection>>) {
		self.0 = link;
	}
}
impl<V> NodeContainer<V> for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	fn node_container_link(&self) -> Option<Link<V>> {
		self.0.as_ref().map(|l| (*l.cid()).into())
	}
}

/// A wrapper for DagLink types that use the BTreeSet type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DagSet<V: Ord>(Option<Link<BTreeSet<V>>>);
impl<V> DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	pub fn create(storage: &mut dyn Storage, items: impl IntoIterator<Item = <Self as DagCollection>::Item>) -> Self {
		Self(Self::to_link(storage, items))
	}

	/// Adds a value to the set.
	pub fn insert(&mut self, storage: &mut dyn Storage, value: V) -> bool {
		let mut set = self.get(storage);
		if set.insert(value) {
			self.set(storage, set);
			true
		} else {
			false
		}
	}
}
impl<V> DagCollection for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = V;
	type Collection = BTreeSet<Self::Item>;

	fn link(&self) -> Option<Link<Self::Collection>> {
		self.0.clone()
	}

	fn set_link(&mut self, link: Option<Link<Self::Collection>>) {
		self.0 = link;
	}
}
impl<V> Default for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn default() -> Self {
		Self(None)
	}
}
impl<V> NodeContainer<V> for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn node_container_link(&self) -> Option<Link<V>> {
		self.0.as_ref().map(|l| (*l.cid()).into())
	}
}

/// A wrapper for DagLink types that use the BTreeMap type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagMap<K, V>(Option<Link<BTreeMap<K, V>>>)
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
}
impl<K, V> DagCollection for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = (K, V);
	type Collection = BTreeMap<K, V>;

	fn link(&self) -> Option<Link<Self::Collection>> {
		self.0.clone()
	}

	fn set_link(&mut self, link: Option<Link<Self::Collection>>) {
		self.0 = link;
	}
}
impl<K, V> Default for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn default() -> Self {
		Self(None)
	}
}
impl<K, V> NodeContainer<(K, V)> for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn node_container_link(&self) -> Option<Link<(K, V)>> {
		self.0.as_ref().map(|l| (*l.cid()).into())
	}
}

#[cfg(test)]
mod test {
	use super::DagSet;
	use crate::{
		types::dag_link::{DagCollection, DagVec},
		DagMap,
	};
	use co_storage::{MemoryStorage, Storage};
	use std::collections::{BTreeMap, BTreeSet};

	struct TestStorage {
		mem_storage: MemoryStorage,
	}
	impl crate::Storage for TestStorage {
		fn get(&self, cid: &libipld::Cid) -> crate::Block {
			self.mem_storage.get(cid).expect("get")
		}

		fn set(&mut self, block: crate::Block) -> libipld::Cid {
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
		let restored_vec = dag_vec.get(&s);
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
		let restored_set = dag_set.get(&s);
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
		let restored_map = dag_map.get(&s);
		assert_eq!(original_map, restored_map);
	}
}
