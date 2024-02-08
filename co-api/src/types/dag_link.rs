use crate::{library::node_reader::node_reader, Block, Storage};
use co_primitives::{DefaultNodeSerializer, NodeBuilder};
use libipld::Cid;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
};

/**
 * Simple trait for converting data into blocks
 */
pub trait IntoBlocks {
	fn into_blocks(&self) -> Vec<Block>;
}

/**
 * Simple trait for recreating data from blocks
 */
pub trait FromBlocks {
	fn from_blocks(cid: &Cid, s: &dyn Storage) -> Self;
}

/**
 * Simple trait for getting the content inside a Dag type
 */
pub trait Content {
	type Item;
	fn content(&self) -> Self::Item;
}

/**
 * A wrapper type for DagLink types that use vectors
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagVec<V: Clone + Serialize> {
	pub content: DagLink<V, Vec<V>>,
}

impl<V: Clone + Serialize> DagVec<V> {
	pub fn new(content: Vec<V>) -> Self {
		Self { content: DagLink::new(content) }
	}
}

impl<V> Content for DagVec<V>
where
	V: Clone + Serialize,
{
	type Item = Vec<V>;
	fn content(&self) -> Self::Item {
		self.content.content()
	}
}

impl<V> IntoBlocks for DagVec<V>
where
	V: Clone + Serialize,
{
	fn into_blocks(&self) -> Vec<Block> {
		self.content.into_blocks()
	}
}

impl<V> FromBlocks for DagVec<V>
where
	V: Clone + Serialize + DeserializeOwned + 'static,
{
	fn from_blocks(cid: &Cid, s: &dyn Storage) -> Self {
		DagVec { content: DagLink::<V, Vec<V>>::from_blocks(cid, s) }
	}
}

/**
 * A wrapper for DagLink types that use the BTreeSet type
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagSet<V>
where
	V: Ord + Clone + Serialize,
{
	pub content: DagLink<V, BTreeSet<V>>,
}

impl<V> DagSet<V>
where
	V: Ord + Clone + Serialize,
{
	pub fn new(content: BTreeSet<V>) -> Self {
		Self { content: DagLink::<V, BTreeSet<V>>::new(content) }
	}
}

impl<V> Content for DagSet<V>
where
	V: Clone + Serialize + Ord,
{
	type Item = BTreeSet<V>;
	fn content(&self) -> Self::Item {
		self.content.content()
	}
}

impl<V> IntoBlocks for DagSet<V>
where
	V: Ord + Clone + Serialize,
{
	fn into_blocks(&self) -> Vec<Block> {
		self.content.into_blocks()
	}
}

impl<V> FromBlocks for DagSet<V>
where
	V: Ord + Clone + Serialize + DeserializeOwned + 'static,
{
	fn from_blocks(cid: &Cid, s: &dyn Storage) -> Self {
		Self { content: DagLink::<V, BTreeSet<V>>::from_blocks(cid, s) }
	}
}

/**
 * A wrapper for DagLink types that use the BTreeMap type
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
{
	pub content: DagLink<(K, V), BTreeMap<K, V>>,
}

impl<K, V> DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
{
	pub fn new(content: BTreeMap<K, V>) -> Self {
		Self { content: DagLink::<(K, V), BTreeMap<K, V>>::new(content) }
	}
}

impl<K, V> Content for DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
{
	type Item = BTreeMap<K, V>;
	fn content(&self) -> Self::Item {
		self.content.content()
	}
}

impl<K, V> IntoBlocks for DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
{
	fn into_blocks(&self) -> Vec<Block> {
		self.content.into_blocks()
	}
}

impl<K, V> FromBlocks for DagMap<K, V>
where
	K: std::cmp::Ord + Clone + Serialize,
	V: Clone + Serialize,
	(K, V): DeserializeOwned + 'static,
{
	fn from_blocks(cid: &Cid, s: &dyn Storage) -> Self {
		Self { content: DagLink::<(K, V), BTreeMap<K, V>>::from_blocks(cid, s) }
	}
}

/**
 * A wrapper type for any iterable data. Will implement FromBlocks and IntoBlocks traits for easy conversion between
 * data and CIDs
 * Types this is mainly used for: Vec, BTreeSet, BTreeMap
 */
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagLink<F, C>
where
	F: Clone + Serialize,
	C: IntoIterator + FromIterator<F> + Clone + Serialize,
{
	_p_data: PhantomData<F>,
	pub content: C,
}

impl<F, C> DagLink<F, C>
where
	F: Clone + Serialize,
	C: IntoIterator + FromIterator<F> + Clone + Serialize,
{
	pub fn new(content: C) -> Self {
		Self { _p_data: PhantomData::default(), content }
	}
}

impl<F, C> Content for DagLink<F, C>
where
	F: Clone + Serialize,
	C: IntoIterator + FromIterator<F> + Clone + Serialize,
{
	type Item = C;
	fn content(&self) -> Self::Item {
		self.content.clone()
	}
}

impl<F, C> IntoBlocks for DagLink<F, C>
where
	F: Clone + Serialize,
	C: IntoIterator<Item = F> + FromIterator<F> + Clone + Serialize,
{
	fn into_blocks(&self) -> Vec<Block> {
		let mut node_builder = NodeBuilder::<F>::new(10, DefaultNodeSerializer::new());
		for item in self.content.clone().into_iter() {
			node_builder.push(item).unwrap();
		}
		node_builder.into_blocks().unwrap()
	}
}

impl<F, C> FromBlocks for DagLink<F, C>
where
	F: Clone + Serialize + DeserializeOwned + 'static,
	C: IntoIterator<Item = F> + FromIterator<F> + Clone + Serialize + Default,
{
	fn from_blocks(cid: &Cid, s: &dyn Storage) -> Self {
		let node_reader = node_reader::<F>(s, cid);
		if let Ok(c) = node_reader.collect() {
			let content = C::from_iter::<C>(c);
			return Self { _p_data: PhantomData::default(), content }
		}

		Self { _p_data: PhantomData::default(), content: C::default() }
	}
}

#[cfg(test)]
mod test {
	use super::DagSet;
	use crate::{
		types::dag_link::{DagVec, FromBlocks, IntoBlocks},
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
		let vec = DagVec::new(vec!["test".to_owned(), "testy".to_owned(), "zesty".to_owned()]);
		let blocks = vec.into_blocks();
		println!("{:?}", blocks);
		let mut s = MemoryStorage::new();
		if let Some(first_block) = blocks.first().cloned() {
			for block in blocks {
				s.set(block).unwrap();
			}
			let ts = TestStorage { mem_storage: s };
			let restored_vec = DagVec::<String>::from_blocks(&first_block.cid(), &ts);
			assert_eq!(vec, restored_vec)
		}
	}

	#[test]
	fn test_set() {
		let mut set: BTreeSet<String> = BTreeSet::new();

		set.insert("test".into());
		set.insert("testy".into());
		set.insert("test".into());
		set.insert("zesty".into());
		let dag_set = DagSet::<String>::new(set);
		let blocks = dag_set.into_blocks();
		println!("{:?}", blocks);
		let mut s = MemoryStorage::new();
		if let Some(first_block) = blocks.first().cloned() {
			for block in blocks {
				s.set(block).unwrap();
			}
			let ts = TestStorage { mem_storage: s };
			let restored_set = DagSet::<String>::from_blocks(&first_block.cid(), &ts);
			assert_eq!(dag_set, restored_set)
		}
	}

	#[test]
	fn test_map() {
		let mut map: BTreeMap<String, String> = BTreeMap::new();

		map.insert("test".into(), "test".into());
		map.insert("testy".into(), "testy".into());
		map.insert("test".into(), "test".into());
		map.insert("zesty".into(), "zesty".into());
		let dag_map = DagMap::<String, String>::new(map);
		let blocks = dag_map.into_blocks();
		println!("{:?}", blocks);
		let mut s = MemoryStorage::new();
		if let Some(first_block) = blocks.first().cloned() {
			for block in blocks {
				s.set(block).unwrap();
			}
			let ts = TestStorage { mem_storage: s };
			let restored_map = DagMap::<String, String>::from_blocks(&first_block.cid(), &ts);
			println!("{:?}", restored_map);
			assert_eq!(dag_map, restored_map)
		}
	}
}
