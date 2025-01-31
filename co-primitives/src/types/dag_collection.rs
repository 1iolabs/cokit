use crate::{Node, NodeContainer, OptionLink};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	cmp::Ord,
	collections::{BTreeMap, BTreeSet},
};

/// Simple trait for creating a DagLink type object
pub trait DagCollection: Sized + Default {
	type Item: Clone + Serialize + DeserializeOwned + 'static;
	type Collection: Default + Clone + IntoIterator<Item = Self::Item> + FromIterator<Self::Item> + Extend<Self::Item>;

	fn link(&self) -> OptionLink<Node<Self::Item>>;
	fn set_link(&mut self, link: OptionLink<Node<Self::Item>>);
}

/// A wrapper type for DagLink types that use vectors
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DagVec<V>(OptionLink<Node<V>>)
where
	V: Clone;
impl<V> DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	pub fn new(link: OptionLink<Node<V>>) -> Self {
		Self(link)
	}
}
impl<V> Clone for DagVec<V>
where
	V: Clone,
{
	fn clone(&self) -> Self {
		Self(self.0)
	}
}
impl<V> Default for DagVec<V>
where
	V: Clone,
{
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

	fn link(&self) -> OptionLink<Node<Self::Item>> {
		self.0
	}

	fn set_link(&mut self, link: OptionLink<Node<Self::Item>>) {
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
pub struct DagSet<V: Ord>(OptionLink<Node<V>>);
impl<V> DagCollection for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = V;
	type Collection = BTreeSet<Self::Item>;

	fn link(&self) -> OptionLink<Node<Self::Item>> {
		self.0
	}

	fn set_link(&mut self, link: OptionLink<Node<Self::Item>>) {
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
pub struct DagMap<K, V>(OptionLink<Node<(K, V)>>)
where
	K: Ord + Clone,
	V: Clone;
impl<K, V> DagCollection for DagMap<K, V>
where
	K: Ord + Clone + Serialize + DeserializeOwned + 'static,
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	type Item = (K, V);
	type Collection = BTreeMap<K, V>;

	fn link(&self) -> OptionLink<Node<Self::Item>> {
		self.0
	}

	fn set_link(&mut self, link: OptionLink<Node<Self::Item>>) {
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
	use crate::{Block, DagCollectionExt, DagMap, DagSet, DagVec, DefaultParams, Storage};
	use cid::Cid;
	use std::collections::{BTreeMap, BTreeSet};

	#[derive(Debug, Default)]
	struct TestStorage {
		records: BTreeMap<Cid, Block<DefaultParams>>,
	}
	impl Storage for TestStorage {
		fn get(&self, cid: &Cid) -> Block<DefaultParams> {
			self.records.get(cid).expect("get").clone()
		}

		fn set(&mut self, block: Block<DefaultParams>) -> Cid {
			let cid = *block.cid();
			self.records.insert(cid, block).expect("set");
			cid
		}
	}
	#[test]
	fn test_vec() {
		let mut s = TestStorage::default();
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

		let mut s = TestStorage::default();
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

		let mut s = TestStorage::default();
		let dag_map = DagMap::create(&mut s, original_map.clone());
		let restored_map = dag_map.collection(&s);
		assert_eq!(original_map, restored_map);
	}
}
